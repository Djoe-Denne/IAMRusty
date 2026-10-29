# ADR-0504 : Les validations fonctionnelles Linux utilisent k6, Chromium pour le navigateur et un parent ops pour le runtime

- Statut : Accepted
- Réalité : Unimplemented
- Date : 2026-10-29
- Décideurs : proposition architecte ratifiée par le lead sous délégation utilisateur long run, révision de mission1 approuvée le 2026-10-29
- Jalon concerné : validation fonctionnelle Linux, transversal, hors Apparatus P0–P6
- SuperSède : aucune
- SuperSédée par : —

Proposition structurelle initiale de l'agent architecte, session
`ses_e776df31bffeRCT8H53iAIlwKu`, examinée et transcrite par le lead pour trois
ordres de mission. La mission1 a ensuite été approuvée ; les missions2/3 ne sont
pas lancées et aucun test k6 n'est encore implémenté à la ratification.

Révision V1 mission1 préparée le 2026-10-29 sur
`989614096110b9813ce67c47a23fad8457c2025a`, session architecte
`ses_e76a83e39ffevzY3AXlh4V9l72`. Le lead a relu intégralement les deux documents
et récupéré leur contenu strictement identique depuis le worktree auteur, puis
**ratifie cette V1 le 2026-10-29** sous la délégation long run de la policy de
mission révision1 approuvée. **Accepted / Unimplemented** : décision admise avant
code, pas qualification Linux ni permission de créer/réveiller un laboratoire.
Les blocages de bail/cible restent explicites ; aucun résultat historique promu.

## Contexte

La campagne authentification a qualifié un parcours Chromium et des oracles
HTTP/Envoy avec des pilotes ponctuels. Le besoin est une structure maintenable
pour rejouer ces scénarios et tester propagation/reprise des droits sous Linux.
Le choix demandé est Grafana k6 ; OHA et les campagnes de performance sont exclus.

## Décision

- `k6/http` pilote les scénarios API ; `k6/browser` utilise un vrai Chromium quand
  cookies, redirections et comportement navigateur constituent l'oracle. Pas d'UI
  artificielle et pas d'équivalence supposée entre client HTTP et navigateur.
- k6 et Chromium s'exécutent sous Linux. V1 : runner hors cluster, dans un espace
  réseau où les origines ratifiées sont réellement accessibles. Un loopback de
  conteneur/pod n'est pas celui de l'hôte ; aucun issuer/callback réécrit pour le
  contourner. La voie Linux du contrôle ops est vérifiée avant usage, pas présumée.
- Le parent ops possède cible, bail, changements temporaires, observations et
  cleanup. k6 produit requêtes, interactions navigateur, assertions et résultats.
  Aucun lancement direct de kubectl/Docker/processus système depuis le script k6,
  aucune API distante générique d'administration créée pour les tests.
- Une VU (utilisateur virtuel) et une itération par scénario fonctionnel. Checks
  obligatoires avec thresholds de réussite intégrale, compteurs de complétude et
  témoins positifs : un scénario vide ou ignoré n'est jamais vert.
- Versions épinglées, modules locaux, artefacts non secrets. Base candidate de la
  recherche retenue pour V1 : k6 v2.3.0 ; version Chromium exacte à épingler puis
  qualifier en mission1, sans la déduire de Chrome149 historique.
  Pas de Grafana Cloud, service payant, Grafana/Prometheus ou k6 Operator requis.
- Aucun `--no-sandbox`, TLS ignoré ou conteneur privilégié par défaut. Échec de
  prérequis bloquant, pas suppression des protections pour obtenir un succès.
- Les UT/IT Rust restent régies par0200–0202 et la politique Windows native du
  dépôt. Linux concerne les scénarios E2E et services déployés, pas un déplacement
  implicite des compilations/tests Rust de développement dans Docker.

### Frontière V1 mission1 retenue

- Seul cas HTTP nominal : **un GET `/iam/health`, HTTP200, corps exact `OK`**,
  sans redirection suivie, seed, authentification ou modification produit. Executor
  `per-vu-iterations`, `vus=1`, `iterations=1` déclarés dans le scénario ; aucun
  raccourci CLI ne remplace cette configuration. Ce smoke prouve la liveness du
  trajet testé, pas la readiness des dépendances ni la campagne OAuth0412.
- Entrée unique `ops/tests/k6/run-linux.py`, modules JavaScript locaux et bibliothèque
  standard Python pour le lanceur ; réutilisation de l'autorité existante
  `ops/deploy/local_full_target.py` (et de son prérequis PyYAML explicite). Pas de
  copie des gardes, nouveau producteur de baux, déployeur, tunnel ou API admin.
- Voie locale retenue : Linux natif, WSL préféré, hors cluster, vers l'exposition
  **existante** local-full `127.0.0.1:18080` seulement si le parent peut l'ancrer et
  la vérifier depuis cet espace réseau. Le contrôle POSIX doit utiliser
  `approved_root()`, `private_file()` et `LocalClusterTargetAnchor.check()` sans
  override de racine/privacy. Copier un bail/kubeconfig Windows, réécrire son
  endpoint/hash ou adopter une ancienne ressource pour satisfaire la garde est
  interdit. Si ce raccordement n'est pas disponible, HTTP reste **INCONCLUSIVE** ;
  sa reprise exige une cible et une autorité existantes conformes, ou un arbitrage
  structurel distinct. Aucun contournement intégré à k6.
- Le parent conserve bail/inventaires, preuves d'identité contemporaines, vérifications
  pré/post et cleanup ; k6 ne reçoit ni bail, kubeconfig, credentials ni commandes
  privilégiées. Un fichier descriptif, un nom, un digest ou une URL ne prouve pas
  la propriété. Les vérifications du cluster ne prouvent pas seules l'origine HTTP.
- CI dédiée `.github/workflows/k6-functional-smoke.yml`, `ubuntu-24.04` x86_64,
  actions par SHA, `contents: read` : outillage et Chromium `about:blank` seulement
  en l'absence de cible autorisée. Aucun accès applicatif/secret pour une PR non
  fiable, aucun accès supposé au loopback Windows. Le vert de cette CI qualifie
  uniquement ses prérequis ; HTTP et clôture de mission restent distincts. Un job
  HTTP requis sans cible échoue, il ne devient pas vert par skip.
- Verdicts **HTTP / Chromium / cleanup** séparés, complétude et confidentialité
  obligatoires. Un exit0 ou un taux de checks à100% sans cas exécuté, résumé frais
  valide et cleanup attesté ne permet pas PASS. Les détails du passage parent→k6,
  des pins/checksums, du selftest et des reprises figurent dans le
  [contrat V1](../contracts/k6-functional-linux.md).

## Conséquences

Trois missions séquentielles : socle/smoke, portage des oracles d'authentification,
puis propagation/reprise des droits sous Linux. Leur [contrat](../contracts/k6-functional-linux.md)
fixe les frontières. Les anciens pilotes restent tant que la parité n'est pas
prouvée. Les preuves historiques et Réalité0412 Implemented localement restent
valides ; elles ne qualifient pas automatiquement le futur portage k6.

Impact **API-contrat / outillage transverse**, limité en mission1 à `ops/tests/k6/`
et au workflow dédié. Aucun port Rust, route, migration, modèle FGA, compose,
monolithe, gitlink ou pipeline existant modifié. La préparation/qualification
runtime reste au parent et ne devient pas une dépendance de déploiement vers les
tests. La nouvelle frontière est précisée dans cette ADR vivante ratifiée pour
la V1, sans nouvelle ADR0505 ni supersession d'une autre Accepted.

## Alternatives écartées

- HTTP seul pour OAuth : perd les garanties de cookie jar/redirection navigateur.
- Copier les scripts Playwright sans adaptation : API inspirée, pas compatibilité
  complète ; k6 n'est pas Node.js et ses bibliothèques ne sont pas interchangeables.
- Un nouvel orchestrateur k6 de cluster : duplique les gardes et augmente les droits.
- Démarrer une stack d'observabilité : non nécessaire au verdict fonctionnel local.
- Convertir toutes les UT/IT en k6 : mauvais niveau, coût et preuve non équivalente.
- Déclarer les performances qualifiées parce que k6 émet des durées : hors mandat.
- Hériter d'un bail Windows ou accepter une URL/manifeste comme autorité Linux :
  ne préserve ni propriété contemporaine, ni identité, ni espace réseau.
- Créer un laboratoire CI ou un serveur factice pour rendre le smoke vert :
  extension de périmètre ou fausse preuve, non autorisée par cette décision.

## Non décidé ici

Dimensionnement, OHA, charge, SLO de production, vendor HSM, nouvelle UX ou nouvelle
autorité d'identité. Une panne révélée reste un défaut à traiter dans son contrat,
pas une permission de modifier le produit ou d'affaiblir les assertions.

## Références et preuve

- [Recherche officielle datée](../missions/20261029-k6-recherche-et-enchainement.md).
- [Sources officielles versionnées et arbitrages V1](../contracts/k6-functional-linux.md#sources-et-choix-retenus-le-2026-10-29),
  [ordre mission1](../missions/20261029-k6-01-socle-smoke.md), `AGENTS.md`.
- `ops/deploy/local_full_target.py` : `approved_root`, `private_file`,
  `LocalClusterTargetAnchor.__init__/local_check/check/_uid_check` ; import par
  d'autres validateurs déjà présent, pas une nouvelle autorité.
- `ops/tests/test_local_full_target.py` :
  `test_valid_fixed_config_and_uid_before_write`,
  `test_stale_uid_one_metadata_get_zero_mutation`,
  `test_kubeconfig_outside_approved_root_zero_commands` examinés, **non exécutés** ;
  fixture avec racine injectée et `privacy=lambda _: True`, donc pas preuve POSIX.
- ADR0200–0202, [0412](0412-oauth-transaction-persistante-liee-navigateur.md),
  [0314](0314-principal-authority-through-openfga.md), [0309](0309-remote-signer.md).
- [État auth qualifié](../contracts/20261029-auth-security-qualified-state.md),
  [contrat exact-route](../contracts/0412-oauth-exact-route-envoy-proof.md).
- Implémentation et exécution k6 : **aucune à la ratification**. Réalité0504
  Unimplemented ; Statut/Réalité des autres ADR inchangés.
