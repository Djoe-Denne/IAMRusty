# ADR-0603 — tranche locale A+B

Statut : Proposed. Réalité : Partial. Canon : docs/adr/0603-tranche-locale-deploy-kind-apparatus-lazaret.md ; voir le fichier ADR.

- Séquence locale-first A+B, pas Accept implicite GKE/Flux/0602 ; aucun Supersede 0600/0601/0008.
- M1–M3 et commandes/preuves conservés comme historiques ; moteur P4/stub/YAML ≠ preuve plateforme métier courante.
- Sources actuelles apps/base + apps/overlays/{kind,kind-demo-monolith,kind-mesh,local-full}, kind/mesh/scripts ops/deploy ; OpenTofu reste ops/cloud/opentofu. D6 rendu112 source/offline, pas runtime.
- Frontière lab dédié : 0606 Proposed/Partial, kind-aiforall-local-full 1+2 autorisé après IT, legacy étranger préservé.
- Compilation root, IT puis E2E réseau/durabilité/reprise/restore distinct encore à prouver ; DEV/SQS volatils/OAuth absent/simulations ne closent aucun constat local.