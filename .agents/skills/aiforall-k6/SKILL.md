---
name: aiforall-k6
description: Prépare, étend et diagnostique les tests fonctionnels k6 AIForAll selon la stratégie IaC-first / fixture-first. À charger pour une campagne k6 locale, un scénario IAM/OAuth/Envoy SAN, les données et dépendances de test, les fixtures Helm, les entrées privées, Chromium, prepare/run/clean ou les preuves et le cleanup. Distingue la campagne native Windows à trois scénarios de la tranche IAM stateless et des lanceurs legacy ; ne réactive pas la CI.
---

# k6 AIForAll — IaC et fixtures d'abord

## 1. Choisir le bon chemin avant toute exécution

- **Trois scénarios conservés, voie locale qualifiée :**
  `ops/tests/k6/native/campaign.ps1` et son chart. Santé HTTPS, OAuth Chromium,
  identité TLS/SAN Envoy. Lire le [fonctionnement courant](references/native-campaign.md)
  pour les entrées, l'alimentation et les oracles.
- **Tranche IAM stateless :** `ops/tests/k6/iam/campaign.ps1`, exposée par
  `just k6-prepare/run/clean iam`. Ses 12 checks de lectures/refus ne couvrent
  ni OAuth positif ni SAN ; cette tranche n'est pas la campagne à trois scénarios.
  Son [README](../../../ops/tests/k6/iam/README.md) garde ses limites propres.
- **Legacy Linux/Windows/WSL :** consommateurs préservés, pas le point de départ
  d'un nouveau scénario. Ne pas rétablir `browser-prerequisite`/`about:blank` comme
  campagne séparée ni déduire une qualification Linux du résultat Windows.

Les recettes `just k6-* iam` ne redirigent **pas** vers `native/campaign.ps1`.
Le [README racine k6](../../../ops/tests/k6/README.md) décrit surtout l'historique ;
privilégier les sources et le README du chemin choisi.

Ces tests sont **fonctionnels**, bornés à une VU/itération par cas. Une demande
de charge, Cloud, password/refresh positif ou nouveau service élargit le périmètre :
définir d'abord ses effets, fixtures et critères, sans l'activer implicitement.

## 2. Concevoir fixture-first, pas scénario puis ménage improvisé

Avant un nouveau cas, expliciter : frontière testée, vraie API utilisée,
dépendances réelles/protocolaires, état initial, données propres au run,
effets aval, oracle positif/négatif et inverse causal sûr.

1. Déclarer les workloads, Services, montages, probes, ressources et selectors
   dans **`ops/tests/k6/native/chart/`**. Alimenter le rendu par les entrées
   explicites ; pas de deuxième déployeur générant des specs en Python/PowerShell.
   Helm rend les fixtures ; ce chemin ne crée pas une release `helm install`.
2. Garder le parent mince et responsable des garanties non déclaratives :
   validation/hachage des entrées, secrets privés, autorité de cible, reçus UID/OID,
   préimages/restauration SAN, lancement et collecte. Les Secrets privés actuels
   sont créés par ce parent, pas placés dans des valeurs publiques versionnées.
3. Injecter les configurations/dépendances de la campagne via les artefacts
   privés et le contexte public fermé transmis par le parent. Les JS décrivent
   stimuli/assertions, pas le provisioning, les secrets ni le cycle du lab.
4. Créer l'état métier via les vraies API du produit quand le parcours est
   précisément ce qu'on teste. Ne pas préinsérer un utilisateur/token pour
   court-circuiter inscription, PKCE, HMAC, signature JWT ou ext-authz.
5. Prouver l'isolation **avant les effets**. Une DB dédiée n'isole pas à elle seule
   les queues, événements, mails et autres consommateurs : supprimer compte/outbox
   après coup ne rappelle pas une notification déjà émise. Pour password/refresh
   ou un nouveau parcours, établir cette frontière avant d'implémenter le stimulus.

Le simulateur IdP Python est une **fixture protocolaire HTTP/TLS** derrière le
vrai OAuthConnect v2, pas un faux IAM ni un remplacement du connecteur. Python
n'est pas interdit par principe ; recréer le framework de déploiement l'est.
Lire le [guide des frontières Ops](../../../docs/guides/ops-declarative-boundaries.md)
pour une nouvelle définition, sans étendre le déploiement nominal pour un test.

## 3. Préparer et exécuter la campagne native

Pour une modification documentaire, ne démarrer aucun runtime. Pour un run réel,
charger `local-runtime-lifecycle` et `aiforall-test-evidence`, lire `AGENTS.md` et
revalider la cible autorisée, ses IDs/UID, les baux et l'état initial.
Le skill n'accorde aucune permission supplémentaire.

Prérequis : Windows x64 non élevé, outils déjà présents et épinglés, Chromium
**complet**, images explicites, lab `kind-aiforall-local-full` déjà disponible,
Postgres/ext-authz et backings requis. Pas de téléchargement/installation implicite.
Le parent ne démarre, ne répare et n'arrête pas le lab ; traiter séparément un
prérequis indisponible. Un nom de nœud ou un digest ne prouve pas la propriété.

La CA publique est installée séparément et durablement dans **CurrentUser\Root**
avec consentement opérateur et empreinte DER vérifiée ; `trust-ca.ps1 -Action verify`
contrôle la confiance. Ne pas importer de clé privée, modifier LocalMachine ou
retirer une CA préexistante. Pas d'attente UI pendant les forwards.

Depuis la racine du dépôt, avec des chemins privés explicites :

```powershell
./ops/tests/k6/native/campaign.ps1 -Action prepare `
  -Config C:/private/native-k6-input.json -Directory C:/private/fresh-campaign

# Seulement après accord réel sur la fenêtre DDL exclusive de la DB du run.
./ops/tests/k6/native/campaign.ps1 -Action run `
  -Directory C:/private/fresh-campaign -ExclusiveDatabaseDdlForThisRun

# Reprendre la fermeture d'un run interrompu, jamais rejouer le même run.
./ops/tests/k6/native/campaign.ps1 -Action clean `
  -Directory C:/private/fresh-campaign -ExclusiveDatabaseDdlForThisRun
```

`prepare` observe la cible, génère et empreinte les artefacts privés ; il ne
crée ni DB/workload ni confiance CA et n'exécute pas les scénarios. Ce n'est
donc pas un rendu entièrement sans accès au lab. Dossier neuf, local, hors Git,
ACL propriétaire, sans reparse point ; sources/entrées modifiées ensuite =
nouveau `prepare` dans un nouveau dossier, pas contournement du hash.

## 4. Recevoir un résultat, pas seulement un exit k6 nul

- Exiger résumés frais liés au run, cas/checks nommés exacts, marqueurs début/fin,
  statut natif, identités et fin des enfants, absence d'overflow et cleanup réel.
- Conserver **`group` dans `systemTags`** : k6 2.3 peut perdre les checks nommés
  du projecteur sans lui, même avec un exit nul. Un compteur agrégé vert ne suffit pas.
- OAuth : TLS vérifié, vraie version Chromium, cookie `__Host-iam-oauth-browser`
  Secure/HttpOnly/SameSite=Lax/Path=/, IdP `localhost` distinct d'IAM `127.0.0.1`.
  Deux ports n'isolent pas un cookie. Garder l'absence de credentials IAM chez l'IdP.
- SAN : bon → mauvais → restauré, refus corrélé sous la même CA et la même
  incarnation Envoy, restauration fonctionnelle **avant suppression** des fixtures.
- Les seuils et deltas exacts figurent dans la référence ; lors d'une extension,
  mettre à jour ensemble cas, schéma de résumé, corrélations parent et tests.
  Ne pas élargir les attendus simplement pour faire passer un run rouge.

Le cleanup concerne les ressources reçues de cette campagne : DELETE avec
précondition UID, arrêt des writers avant DROP de la DB avec OID et zéro connexion,
forwards/processus joints. L'accord DDL reste nécessaire : contrôle OID puis DROP
par nom ne sont pas atomiques. Pas de FORCE, purge, adoption, suppression par
labels seuls ou arrêt du lab partagé. Une restauration/fermeture non prouvée
reste FAIL/INCONCLUSIVE, pas « réparée » en effaçant les preuves.

Ne publier que des projections allowlistées avec commit/empreintes. Les inputs,
patchs, headers, URLs OAuth avec query, tokens, cookies, clés, dumps et logs bruts
restent privés ; jamais de zip récursif du dossier de campagne.

## 5. Statut et validation proportionnée

- Preuve locale de référence : **54a19ba**, run
  `e3337dff24eb470881233fdfad6f6022`, [rapport](../../../docs/missions/20261112-k6-three-native-results.md).
  Les trois scénarios sont qualifiés sur cette slice, pas sur tout le lab/production.
- Le nettoyage Python **fc5e29b** retire seulement une constante du simulateur :
  tests unitaires rejoués, **pas** nouvelle qualification E2E du tree modifié.
- CI GitHub et retrait du legacy sont **reportés**. Ne créer aucun secret GitHub,
  exporter de clés locales ni réactiver un workflow sans demande distincte.
- Le contrat [parité native](../../../docs/contracts/k6-three-scenario-native-parity.md)
  reste **Proposed** ; un run réussi ne le promeut pas en ADR Accepted.

Après changement de code : tests ciblés de rendu/refus, projecteur et parent,
plus simulateur si touché ; vrai run seulement dans le mandat autorisé. Après
changement du skill seul : vérifier liens, commandes et cohérence avec les sources,
sans campagne réelle. Rendre chemin choisi, sources/commit, validations réellement
faites, limites et cleanup/keepalive ; ne pas transformer un skip en PASS.
