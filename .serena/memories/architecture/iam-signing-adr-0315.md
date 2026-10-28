# Transport sécurisé remote — proposition ADR alternative non intégrée

ADR0315 proposée par pool-4 reste sur sa branche, non intégrée/non ratifiée ; pas une ADR canonique du checkout principal.

- Canon : ADR0309 Accepted/Partial et0311 Accepted/Implemented ; docs/contracts/remote-signer-secure-transport.md ratifié par le lead le2026-10-28 et désormais Accepted/Partial. Voir ces fichiers.
- HTTPS Verified, HTTP uniquement modes locaux explicites loopback littéral, zéro redirect/proxy ambiant ; Sign4096B/public16384B, erreur0B applicatif, délai10s incluant WorkloadIdentity.
- Code livré17c9cfd, sept fichiers intégrés avec hashes identiques. Tests Windows natifs config29/protocole19/TLSclient2 PASS ; défaut allowed_algorithms vide conserve RS256 effectif, HS256/mix refusés. Pas de fallback.
- fmt trois packages IAM et Clippy ciblés PASS ; rootfmt FAIL environnement OS206, pas PASS global. SDK d60a88b au test remote ; combinaison avec nouveauSDK72845d9 et revue globale restent à qualifier.
- Pas de nouvelle ADR0315 : précision du provider existant sans nouveau port/protocole/stockage. Proposition Sign16KiB non retenue, branche préservée.
- Aucun Envoy/HSM/cloud/production qualifié par ces tests ; aucun Docker/Kind/WSL démarré pour eux.