# Campagne native — sources, alimentation et réception

État vérifié le 12 novembre 2026, base `fc5e29b`. Revalider les sources avant
un changement : ce document explique le code livré, pas un nouveau framework.
Tous les chemins ci-dessous sont relatifs à la racine du dépôt.

## Sources à consulter selon la tâche

- Routage et prérequis : `ops/tests/k6/native/README.md`.
- Préparation, mutation, scénarios et oracles : `native/campaign.ps1`.
- Pins, cible, création/UID, SQL et forwards : `native/helpers.ps1`.
- Specs publiques : `native/chart/templates/fixtures.yaml`.
- Pins outils : `config/tools.windows.lock.json` ; extraction complète depuis
  archive locale épinglée : `native/provision-chromium.ps1` ; confiance publique :
  `native/trust-ca.ps1`. Ces chemins sont sous `ops/tests/k6/`.
- Contexte JS : `ops/tests/k6/lib/campaign-target.js` ; résumé : `lib/summary.js` ;
  schémas/deltas du parent : `lib/windows-auth.ps1` ; processus : `lib/windows-native.cs`.
- Stimuli : `ops/tests/k6/scenarios/smoke/iam-health.js`,
  `scenarios/auth/oauth-browser.js`, `scenarios/auth/envoy-san.js`.
- Fixture fournisseur : `ops/tests/support/oauth-idp-simulator.py`.
- Tests : `ops/tests/k6/tests/test_native_campaign.py`, autres tests k6 aux
  frontières concernées, `ops/tests/test_oauth_idp_simulator.py`.

## 1. Ce qui alimente la campagne

Le JSON **privé hors Git** passé à `prepare -Config` fournit :

- `operator_directory` : sources opérateur complètes `apps/iam/`,
  `apps/envoy-mesh/envoy.yaml`, `pki/` ; pas une recherche du dossier « le plus récent ».
- `kubeconfig` : fichier explicite pour le contexte fermé
  `kind-aiforall-local-full`, namespace `aiforall-local-full`.
- `tools` : `k6`, `chromium`, `kubectl`, `docker`, `helm`, `openssl` avec
  `path` absolu et `sha256`. Chromium ajoute `bundle_manifest`/`bundle_sha256` ;
  le parent vérifie aussi les fichiers compagnons. Un chrome.exe seul ne suffit pas.
  Pins actuels : k6 2.3.0, CfT 156.0.8078.73 ; pas le Chrome habituel de l'utilisateur.
- `images` : `iam`, `oauth`, `envoy`, `idp` explicites. Images applicatives
  déjà chargées dans Kind ; IdP sur image Python officielle épinglée par digest.
  Kind ne compile pas ; ne pas déclencher un build lors du test.

`prepare` crée un `run_id` et dérive les noms de ressources `k6-<run>` et de DB
`k6_<run>`. Il vérifie la source DB nominale **sans l'utiliser comme cible
d'écriture**, puis dérive la connexion vers la DB dédiée. Il prépare :

- configuration IAM d'origine + overlay DB/connecteur, clés de signature compatibles
  avec le nominal, sources TLS originales inchangées ;
- client/secret OAuth et HMAC **propres à la campagne**, catalogue OAuthConnect v2
  `[[providers]]`, instance `gitlab`, PKCE et transport TLS vérifié ;
- `provider.json` pour la fixture IdP, certificats OAuth/IdP à durée courte signés
  localement, matériel IAM bon/mauvais/restauré ; aucune rotation de CA nominale ;
- rendu Helm, `target.json` public fermé, `k6-options.json` vide explicite,
  empreintes des sources, entrées et artefacts dans `state.json` privé.

La clé de CA existante sert localement à signer ; elle n'est pas importée dans
le magasin Windows ni transmise au navigateur ou à GitHub. Ne pas afficher les
valeurs des TOML/JSON privés ni publier leurs contenus pour documenter les inputs.

## 2. Qui crée quoi au run

`run` revalide les empreintes, la cible et les pins, puis crée la DB dédiée sous
fenêtre DDL exclusive. IAM initialise son propre schéma au démarrage ; le parent
ne seed pas les utilisateurs en SQL. Il crée huit Secrets privés de configuration/TLS,
puis les dix objets du rendu Helm : deux ConfigMaps, quatre Deployments et quatre
Services. Chaque création est reçue par UID ; une collision est un refus, pas une adoption.

La slice contient les **vrais IAM et OAuthConnect v2**, son propre Envoy et l'IdP
protocolaire. Postgres, ext-authz/OpenFGA et autres backings configurés restent
partagés avec le lab. Ne pas la présenter comme une pile hermétique ou une fixture
testcontainers ; vérifier les effets transverses de toute extension.

Le chart dérive Envoy de la source opérateur et ne change que la destination réseau
IAM vers le Service fixture. Autorités TLS, routes et politiques ne sont pas
assouplies. Probes TCP : présence seulement, pas preuve métier ni trafic polluant
les compteurs. Le parent vérifie la destination chargée et la compatibilité JWKS.

Trois forwards possédés et vérifiés avant stimulus :

- `127.0.0.1:18443` → Envoy fixture 10000, origine IAM HTTPS ;
- `127.0.0.1:18445` → IdP fixture 8080, autorisation navigateur sur
  **`https://localhost:18445/oauth/authorize`** ;
- `127.0.0.1:19901` → Admin Envoy 9901, observations privées seulement.

Le certificat IdP inclut DNS:localhost. Le cookie IAM HTTPS est host-only,
`__Host-iam-oauth-browser`, Path=/, Secure/HttpOnly/SameSite=Lax/Max-Age=600.
Séparer les hosts, pas uniquement les ports, protège le fournisseur du cookie IAM.

## 3. Comment les JS reçoivent cible et données

Le parent construit un environnement fermé via `New-K6Environment` : run ID,
chemin du résumé, `K6_ORIGIN`, `K6_CAMPAIGN_FILE`, version/browser attendus pour
OAuth, phase SAN pour les cas concernés. Pas d'options K6/proxy/debug hérités
ni de configuration utilisateur implicite. Les scénarios ne reçoivent pas les
credentials opérateur et n'interrogent pas Kubernetes.

`campaign-target.js` valide strictement version/profil/run/origines/issuer/audience
de `target.json`. Le fallback sans fichier est **legacy HTTP**, pas une variante
native autorisée quand un input manque. Ne pas lancer les JS natifs à la main en
omettant ce contexte pour « faire marcher » le test.

Les données métier viennent du parcours : l'IdP produit une identité déterministe
liée au run (`<run_id>@example.invalid`), le navigateur fait l'inscription OAuth
et sa complétion avec un username lié au run, puis un **nouveau login** et un relink
authentifié. L'access token vient d'IAM, passe par Envoy ; ni JWT fabriqué ni faux
validateur. Trois refus mesh complètent le parcours. La fixture contrôle codes
à usage unique, client/redirect et S256, et n'exporte que des compteurs fermés.

## 4. Oracles et fermeture

Pour la campagne actuelle, cinq phases pour trois scénarios :

- santé : 200/corps `OK`, un GET, deux checks ;
- SAN : bon303 → mauvais503 → restauré303 sans suivre les redirects, deux checks
  par phase ; delta `fail_verify_san=1`, pas une erreur CA quelconque ;
- OAuth Chromium : 26 checks, version réelle, identité/login/relink, cookies,
  headers, PKCE et refus. Total campagne : **34 checks réussis** attendus ;
- corrélations parent OAuth : IAM+7, ext-authz+4, authorize/token/profile/PKCE+3
  chacun, sans erreur transport ; DB users+1/outbox+0 ; même incarnation Envoy.

`lib/summary.js` ne publie pas les métriques brutes ; `group` dans `systemTags`
rend les checks nommés disponibles. Une nouvelle assertion exige un schéma parent
et des tests cohérents, pas une tolérance sur les cardinalités.

Restaurer le SAN bon et prouver son retour, arrêter/supprimer les writers possédés,
vérifier OID/zéro connexion puis DROP de la DB, supprimer les autres objets par UID,
joindre les forwards et tous les processus k6/Chromium. Pas de GET Secret via
`kubectl wait` ; le parent évite aussi ce GET pour restaurer par préimages privées.
Restauration impossible = fixtures retenues et échec explicite. Ne pas effacer le
verrou après interruption sans examen des preuves/processus.

La confiance CA opérateur préexistante reste en place. Le mode temporaire
`-TrustLabCaForThisRun` est une option distincte exigeant consentement ; pas le
mode par défaut ni une raison de retirer la confiance durable. Laisser le lab
au propriétaire/bail courant et appliquer `local-runtime-lifecycle` pour ses
ressources propres, sans stop global Docker/WSL sur keepalive.

## 5. Régressions utiles, sans qualification fictive

Depuis la racine, sous Windows et via Context Mode pour les sorties longues :

```powershell
python -B -m unittest discover -s ops/tests/k6/tests -p test_native_campaign.py
python -B -m unittest ops.tests.test_oauth_idp_simulator
```

Ce sont des tests de rendu/parent/protocole/outillage, **pas** un run k6 contre
le lab. Le vrai k6 peut être utilisé sans réseau par un oracle du projecteur.
Le dernier bilan complet de régression connu est 195 PASS/8 POSIX skips +9 IdP ;
ne pas recopier ces nombres comme résultat d'un nouveau run.

Cas de réception du skill, à relire après modification :
1. « Relance les trois scénarios » → chemin native, inputs explicites et autorisés,
   CA prétrustée, DDL consenti, cinq phases et cleanup ; pas `just ... iam`.
2. « Ajoute password/refresh positif » → d'abord effets aval et fixtures/inverse ;
   pas d'écriture nominale puis DELETE SQL pour prétendre annuler les notifications.
3. « k6 exit0 mais checks vides / cookie absent / Chromium SxS » → vérifier
   `group`, contexte/cookie racine et bundle complet ; aucun bypass TLS/sandbox,
   aucun changement d'attendu OAuth pour masquer l'erreur, aucun démarrage automatique.
