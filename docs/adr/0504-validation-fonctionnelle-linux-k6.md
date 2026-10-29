# ADR-0504 : Les validations fonctionnelles Linux utilisent k6, Chromium pour le navigateur et un parent ops pour le runtime

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-10-29
- Décideurs : utilisateur à la ratification du contrat détaillé ; choix k6 demandé explicitement
- Jalon concerné : validation fonctionnelle Linux, transversal, hors Apparatus P0–P6
- SuperSède : aucune
- SuperSédée par : —

Proposition structurelle de l'agent architecte, session
`ses_e776df31bffeRCT8H53iAIlwKu`, examinée et transcrite par le lead. La demande
porte sur trois ordres de mission, pas leur exécution. Aucun test k6 implémenté.

## Contexte

La campagne authentification a qualifié un parcours Chromium et des oracles
HTTP/Envoy avec des pilotes ponctuels. Le besoin est une structure maintenable
pour rejouer ces scénarios et tester propagation/reprise des droits sous Linux.
Le choix demandé est Grafana k6 ; OHA et les campagnes de performance sont exclus.

## Décision proposée

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
  recherche : k6 v2.3.0 stable ; version Chromium exacte à qualifier en mission1.
  Pas de Grafana Cloud, service payant, Grafana/Prometheus ou k6 Operator requis.
- Aucun `--no-sandbox`, TLS ignoré ou conteneur privilégié par défaut. Échec de
  prérequis bloquant, pas suppression des protections pour obtenir un succès.
- Les UT/IT Rust restent régies par0200–0202 et la politique Windows native du
  dépôt. Linux concerne les scénarios E2E et services déployés, pas un déplacement
  implicite des compilations/tests Rust de développement dans Docker.

## Conséquences

Trois missions séquentielles : socle/smoke, portage des oracles d'authentification,
puis propagation/reprise des droits sous Linux. Leur [contrat](../contracts/k6-functional-linux.md)
fixe les frontières. Les anciens pilotes restent tant que la parité n'est pas
prouvée. Les preuves historiques et Réalité0412 Implemented localement restent
valides ; elles ne qualifient pas automatiquement le futur portage k6.

## Alternatives écartées

- HTTP seul pour OAuth : perd les garanties de cookie jar/redirection navigateur.
- Copier les scripts Playwright sans adaptation : API inspirée, pas compatibilité
  complète ; k6 n'est pas Node.js et ses bibliothèques ne sont pas interchangeables.
- Un nouvel orchestrateur k6 de cluster : duplique les gardes et augmente les droits.
- Démarrer une stack d'observabilité : non nécessaire au verdict fonctionnel local.
- Convertir toutes les UT/IT en k6 : mauvais niveau, coût et preuve non équivalente.
- Déclarer les performances qualifiées parce que k6 émet des durées : hors mandat.

## Non décidé ici

Dimensionnement, OHA, charge, SLO de production, vendor HSM, nouvelle UX ou nouvelle
autorité d'identité. Une panne révélée reste un défaut à traiter dans son contrat,
pas une permission de modifier le produit ou d'affaiblir les assertions.

## Références et preuve

- [Recherche officielle datée](../missions/20261029-k6-recherche-et-enchainement.md).
- ADR0200–0202, [0412](0412-oauth-transaction-persistante-liee-navigateur.md),
  [0314](0314-principal-authority-through-openfga.md), [0309](0309-remote-signer.md).
- [État auth qualifié](../contracts/20261029-auth-security-qualified-state.md),
  [contrat exact-route](../contracts/0412-oauth-exact-route-envoy-proof.md).
- Implémentation et exécution k6 : **aucune**. Statut/Réalité des ADR existantes
  inchangés par cette proposition.
