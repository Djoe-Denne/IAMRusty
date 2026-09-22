Jalon : Cloud-portable (hors Apparatus P0–P6).
Chemin : `docs/adr/0601-cluster-trust-namespaces-standalones.md`.
Statut : Proposed. Réalité : Unimplemented.
- Unité cluster V1 = 4+1 Deployments (iam, hive, manifesto, telegraph + lazaret) + sentinel-sync B/C ; monolithe = laptop/démo seulement.
- Namespaces `aiforall-*` identiques ; tenancy = Hive+FGA ; ns-per-tenant interdit V1.
- 4 SA conceptuels ; colocation admit+sign OK ; split admit/signer reste dans 0008.
- Ferme le Non décidé K8s de 0404 sans éditer 0404. P4 se loge dans `aiforall-apparatus` + `deploy/p4/`.
Voir le fichier ADR.