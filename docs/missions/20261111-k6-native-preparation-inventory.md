# k6 : prérequis réels et retrait du setup Python — inventaire du 11 novembre

## Périmètre

Première étape de la migration demandée : conserver k6 et ses scénarios JavaScript,
sortir les préparations de l'ancien framework Python, supprimer ce qui devient
inutile **après** migration des consommateurs. Cet inventaire ne livre pas encore
les commandes de préparation/exécution/nettoyage et ne qualifie aucun nouveau run.

Source observée : `master@8da70f7`. Aucun test écrit ou exécuté, aucun lancement
k6/Chromium, aucune mutation du lab, des clés, des profils ou de la DB.

### Réorientation discutée après l'inventaire

L'utilisateur a proposé de **réécrire entièrement les tests k6**, plutôt que de
migrer leurs anciens scripts, avec pour objectifs les fonctionnalités, les cas
d'erreur et la sécurité du service. Conserver k6 n'impose donc pas de conserver
les quatre JS ni leurs26 checks historiques. L'inventaire ci-dessous décrit
l'existant ; ce n'est pas un critère obligatoire de parité.

La recommandation devient une **petite suite orientée produit**, avec réemploi
ponctuel seulement s'il est utile. Le choix réutilisation/réécriture et le premier
périmètre — IAM ou plateforme — restent en attente de confirmation. Aucune
suppression ou écriture de scénario n'a été entreprise. Les tests Rust restent
intacts. Les protections de cible, TLS, secrets, erreurs et cleanup demeurent
nécessaires quel que soit le langage.

## Les quatre scénarios existants

### 1. Santé IAM — premier candidat à migrer

[`iam-health.js`](../../ops/tests/k6/scenarios/smoke/iam-health.js) vérifie un GET
`/iam/health`, statut200 et corps `OK`, avec un seul cas/itération et des seuils.
Il dépend d'un IAM accessible, pas d'un profil utilisateur en DB, d'un fournisseur
OAuth ou de nouvelles clés de test. Son origine est actuellement **HTTP18080**.

Préparation externe cible : vérifier le bon lab, sa disponibilité et sa liaison
au port-forward ; sélectionner la CA attendue et un répertoire privé de campagne.
Adapter ensuite le scénario à l'edge **HTTPS18443**, en conservant checks, seuils,
résumé de campagne et refus des faux positifs. La confiance TLS native de la
version Windows de k6 reste à vérifier : pas de `insecureSkipTLSVerify` implicite.
Nettoyage : uniquement forward et fichiers appartenant à cette invocation.

### 2. Prérequis navigateur — distinct du test applicatif

[`browser-prerequisite.js`](../../ops/tests/k6/scenarios/smoke/browser-prerequisite.js)
ouvre `about:blank`, vérifie6*7, ferme page/contexte et peut contrôler la version
Chrome156. Il n'a besoin **ni de Kind, ni de DB, ni d'IAM**.

Préparation externe cible : binaires/version compatibles et conditions normales
du navigateur, sans désactivation du sandbox. Ses diagnostics Linux/Windows et
captures privées restent des outils optionnels, pas un prérequis du smoke HTTP.
Leur retrait doit être explicite avec leurs workflows ; pas une disparition des
garanties de sécurité obtenue par un lancement direct moins contrôlé.

### 3. Identité serveur Envoy — campagne mutatrice isolée

[`envoy-san.js`](../../ops/tests/k6/scenarios/auth/envoy-san.js) émet un témoin HTTP
par phase : bon certificat303, mauvais SAN503, certificat restauré303. Il ne
prépare pas les certificats et n'interroge pas l'Admin lui-même.

Préparation externe requise : un profil de test avec connecteur GitLab opérationnel,
deux feuilles de même CA mais identité différente, cible et Secret précis,
snapshots des compteurs Envoy d'une même incarnation. Une phase externe doit
poser le mauvais certificat, attendre le rollout et **restaurer l'original même
après échec**, en conservant les preuves de mutation/restauration.

Le mauvais503 seul ne prouve rien sur le SAN. Une traduction en quelques appels
kubectl sans contrôle de cible/restauration serait une régression, pas un nettoyage.
Ne jamais faire cette campagne sur le lab nominal par simple convention de nom.

### 4. OAuth navigateur — profil de test dédié à redéfinir

[`oauth-browser.js`](../../ops/tests/k6/scenarios/auth/oauth-browser.js) couvre
inscription, login, relink et refus d'alias avec **26 vérifications**, plus les
observations/corrélations du parent. Il crée déjà son utilisateur unique **par API**,
avec une adresse dérivée du run_id ; aucune fixture SQL de profils utilisateurs
n'est actuellement déclarée pour ce scénario.

Préparation externe requise : fournisseur IdP simulé, connecteur réel, HMAC local,
callbacks, issuer, cookies et routes cohérents, compteurs fournisseur/PKCE et
surfaces publiques précisément exposées. L'ancien script impose HTTP18080,
issuerHTTP8080, fournisseur18081 et cookiesSecure=false : il faut une migration
de contrat vers HTTPS, pas seulement remplacer une URL.

Le [simulateur IdP](../../ops/tests/support/oauth-idp-simulator.py) est une fixture
protocolaire de200lignes, distincte d'un déployeur. Sa fonction reste nécessaire
si le scénario sans compte externe est conservé. Réécrire ces200lignes dans un
autre langage ne constitue pas à lui seul une suppression de complexité.

## Ce qui empêche de supprimer immédiatement les wrappers

- [`windows-auth.ps1`](../../ops/tests/k6/lib/windows-auth.ps1) lance explicitement
  [`windows-auth-ops.py`](../../ops/tests/k6/lib/windows-auth-ops.py), qui s'appuie
  sur `local_full_target.py` pour l'admission de cible, les snapshots, le forward,
  la mutation des certificats et leur restauration. L'ancien Secret global
  `platform-mesh-certs` n'est pas le câblage actuel à Secrets par service.
- L'ancien profil `local-full-oauth-proof` est documenté, mais son ancien chart et
  ses profils ont été retirés. Le lab natif a les connecteurs désactivés par défaut.
  Il manque donc un profil de test adapté, pas juste un remplacement de Python.
- [`run-linux.py:70–91`](../../ops/tests/k6/run-linux.py) refuse explicitement la
  liaison HTTP après son contrôle de cluster (`TARGET_BINDING_UNQUALIFIED`). La CI
  [`k6-functional-smoke.yml`](../../.github/workflows/k6-functional-smoke.yml)
  exécute de l'outillage et des prérequis navigateur ; cela ne qualifie pas IAM.
- [`summary.js`](../../ops/tests/k6/lib/summary.js) fournit déjà une projection
  courte des checks/cas/itérations sans exporter tokens, URL ou corps. À conserver,
  avec refus d'un résumé vide, ancien, incomplet ou d'un exit k6 non nul.

Le bloc Python k6 représente20fichiers/6732lignes :6fichiers de harness2736lignes
et14tests de cet outillage3996lignes. Ce ne sont pas6732lignes de scénarios métier.
Ces chiffres ne sont **pas** une estimation du volume supprimable sans migration.

## Ordre de travail initial proposé — à réviser selon le choix

Si la réécriture est retenue, commencer par une petite matrice des fonctionnalités,
refus et contrôles de sécurité réellement exposés par le service choisi, plutôt
que porter les anciens scénarios. Le smoke HTTPS n'est alors qu'un raccord
technique, pas la couverture fonctionnelle/sécurité demandée. La liste ci-dessous
est le premier découpage de migration, pas une décision d'exécution :

1. **Smoke IAM HTTPS** : un chemin utile et petit, préparation native sans seedDB,
   k6 direct, contrôle des résultats et nettoyage des seuls objets créés.
2. **Prérequis navigateur indépendant** : ne pas imposer tout son diagnostic à
   chaque test HTTP ; décider séparément des campagnes historiques à conserver.
3. **Profil OAuth dédié et scénario HTTPS**, en conservant ses vérifications et
   corrélations, sans comptes fournisseurs réels ni seedDB inventé.
4. **Campagne SAN isolée**, avec mutation/restauration et critères préexistants.
5. Supprimer les anciens adaptateurs, contrats d'entrée obsolètes, étapes CI et
   tests exclusivement associés **lorsque leurs consommateurs sont migrés ou
   explicitement abandonnés**. Les deux helpers Ops ont aussi des consommateurs
   hors k6 : leur retrait est une disposition distincte, pas un effet automatique.

Interface envisagée : `just k6-prepare <profil>`, `just k6-run <profil>`,
`just k6-clean <profil>` ; noms et implémentation restent à livrer. Le futur contexte
privé transporte endpoints et identifiants des fixtures, pas une nouvelle autorité
autodéclarée ou un protocole de baux maison. SQL via psql seulement si une fixture
réelle le nécessite ; OpenSSL/CLI Bao/FGA/kubectl/Helm selon leur rôle. Pas de moteur
Terraform pour quelques données ni de traduction globale Python→PowerShell.

## Conditions de reprise

Les contrats k6 existants gardent leur statut ; ce document ne les remplace pas
implicitement et n'affaiblit pas leurs garanties. Les choix de migration sont
bornés par le [contrat proposé](../contracts/k6-native-campaign-preparation.md).
L'indisponibilité du port API du lab signalée le11novembre reste un blocage distinct
avant une exécution réelle, pas un motif pour démarrer/réinitialiser le runtime
pendant cet inventaire. Aucun scénario n'a été déclaré migré ou validé ici.
