# Local-full — ADR0608

- Canon : `docs/adr/0608-recreer-local-full-pour-oauthconnect.md` ; Statut : Proposed ; Réalité : Partial au 2026-11-09 : lab reconstruit, 18 Deployments prêts, HTTPS JWKS200 et login simulé303 ; navigateur complet NOT RUN. Voir le fichier ADR.
- Autorisation opérationnelle utilisateur 2026-11-08 : recréer seulement `aiforall-local-full` 1+2 ; perte DB/PVC/coffre locale acceptée, pas Accept global ni procédure prod.
- Préserver legacy/autres clusters, volumes inconnus/PostgreSQL opaque, registre OCI externe, images/caches, credentials et PKI privée complète courante. Nouveau bail v2/kubeconfig privé ; identité attestée avant destruction et ancre avant API.
- Réemploi explicite des neuf images/artefact via build-manifest/kind load après provenance ; premier deploy `--oauth-connect` github+simulated sans préservation TLS ancienne ni compatibilité réseau. Zéro force-conflicts/skip TLS/prune global.
- Contrat : `docs/contracts/local-full-oauthconnect-recreation.md` ; parent tient preuves et disposition des correctifs. Dette SSA/convergence, expiration Docker OFF, migration données/HA/cloud/GitLab non qualifiées ; aucun PASS runtime par cet écrit.
- Exception manuelle user du 9 novembre : peer IPIP mesuré `10.244.230.64/32` TCP10000 sur allow-envoy, CAS UID/RV/spec, HTTPS validé. Non qualifiée par le helper spécialisé historique, non automatisée dans deploy/up ; nouvelle mesure/autorisation si changement ou reconstruction. Pas de promotion Accepted.
