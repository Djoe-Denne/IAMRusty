# ADR-0606 — frontière lab local-full dédié

Statut : Proposed. Réalité : Partial. Jalon : Local-full / Vague4 locale. Canon : docs/adr/0606-local-full-kind-isole.md ; voir le fichier ADR.

- Kind dédié aiforall-local-full / kind-aiforall-local-full 1CP+2workers, namespaces locaux isolés/deps in-Kind ; autorisation de périmètre, pas Accept technique implicite.
- Legacy/protégés intacts ; IDs loués exacts, mismatch/propriété inconnue ⇒ STOP, aucune suppression/recreate/adoption automatique.
- S-11 MEDIUM ouvert : leasev2 CP exact→mapping6443/endpoint→UID kube-system + kubeconfig fixe possédé ; wrong-port rejette avant tout API GET. Contrat§15, garde non implémentée/provée.
- D6 render112/CA-public-only/mappingHTTP18080 = source/offline, pas runtime/TLS ; PVC node-local ≠ HA, DEV/SQS volatils/SMTP-catalog simulés/OAuth désactivé restent limites.
- Compilation/units root, IT puis E2E full autorisé après intégration avec LIVE/SIMULÉ/NOT RUN/restore distinct ; SDK22/J3 ne closent pas full. 0404/0500/0600–0602 intacts.