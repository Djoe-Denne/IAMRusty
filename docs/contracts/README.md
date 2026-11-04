# Contrats d'implémentation

Décisions structurelles écrites par l'architecte (`architecte`) : API, ports,
formats d'événements, contrats de migration, invariants. **Canon secondaire** :
un contrat découle d'une ADR (`docs/adr/`) ou prépare une ADR `Proposed`.

Conventions :

- Nom : `<slug>.md` (anglais ou français, kebab-case, sans date — l'historique
  vit dans git).
- Tout contrat est **référencé** : lien depuis l'ADR concernée, ou depuis la
  ligne d'index de `docs/adr/README.md` si l'ADR n'existe pas encore.
- Cycle : un contrat ne modifie pas une ADR `Accepted` ; s'il la contredit,
  l'architecte ouvre la mise à jour d'ADR (Proposed) et signale l'écart.
- Le contrat est le vecteur du travail : `implementer`/`hard-implementer`
  reçoivent le chemin du fichier, pas un résumé de chat.

Contrats :

- [`apparatus-knative-runtime.md`](apparatus-knative-runtime.md)
  — cible **Acceptée le 2026-10-20** de l’ADR 0012 : Serving/KPA par binding, activation à zéro, identité et gates bloquantes ; Réalité Unimplemented, pas de GO implémentation tant que les preuves de campagne manquent (pins B5 via spike séparé).

- [`20261018-m6-proof-hardening.md`](20261018-m6-proof-hardening.md)
  — contrat test-side m6 : garde de cible, oracles CLI, canary et captures ; pas de promotion ADR.

- [`0412-oauth-exact-route-envoy-proof.md`](0412-oauth-exact-route-envoy-proof.md)
  — preuve E2E exact-route de l'ADR 0412. Ne promeut pas la Réalité.

- [`ops-kind-runtime-validation-separation.md`](ops-kind-runtime-validation-separation.md)
  — séparation ops runtime/validation : nominal local-full, interface validation (smoke/curl/E2E), bail opérateur. Annexe manifeste de tri : [`ops-kind-runtime-validation-manifest.md`](ops-kind-runtime-validation-manifest.md). Proposed, lien secondaire vers l'ADR 0606 sans nouvelle ADR ; aucune commande exécutée.

- [`telegraph-notification-outbox.md`](telegraph-notification-outbox.md)
  — Proposed : notification/livraison/outbox same-txn, disabled/no-op suspendu, cycle de vie SQS cancellation-safe autorisé (sous-lot T2 préalable), oracles AC1–AC7 ; prépare l'ADR complémentaire 0313, sans modifier 0301.

- [`local-full-github-additive-activation.md`](local-full-github-additive-activation.md)
  — Proposed / Partial : composition privée GitLab+GitHub et lectures cohérentes
  livrées ; activation initiale du full existant, provenance courante et promotion
  ready non livrées. Aucun déploiement ou départ E2E déduit de cette tranche.

- [`local-full-iam-preimage-qualification.md`](local-full-iam-preimage-qualification.md)
  — Proposed / Unimplemented : qualification initiale et post-snapshot IAM.
  Exception locale de lecture ciblée choisie le 2026-11-04 ; méthode d'ancre et
  tests attendent admission native. Identité courante non prouvée = activation
  toujours bloquée avant écrit, aucun droit de lecture général.

- [`local-full-telegraph-e2e-inverse.md`](local-full-telegraph-e2e-inverse.md)
  — Constat sourcé : inverse actuel non qualifié, garde avant signup conservée.
  Le §8 fixe l'exception locale de rétention consentie pour un seul run, encore
  non implémentée ; le défaut strict demeure. Les solutions applicatives durables
  restent Proposed, sans correctif Rust ou lancement implicitement autorisé.
