# Proposition alternative non intégrée — transport remote

Le document ADR0315 proposé par pool-4 le2026-10-28 est resté sur sa branche isolée, non intégré et non ratifié. Il ne constitue pas une ADR canonique présente dans docs/adr sur le checkout principal.

- Canon retenu : ADR0309 Accepted/Partial et ADR0311 Accepted/Implemented, avec docs/contracts/remote-signer-secure-transport.md explicitement ratifié par le lead sous délégation utilisateur le2026-10-28 ; obligations nouvelles Unimplemented. Voir ces fichiers.
- HTTPS Verified, HTTP uniquement modes locaux explicites loopback littéral, zéro redirect/proxy ambiant ; Sign4096B/public16384B, erreur0B applicatif, échéance10s incluant WorkloadIdentity.
- Choix de ne pas créer ADR0315 : précision du provider existant sans nouveau port/protocole/stockage. La proposition alternative16KiB pour Sign n’est pas retenue. Pas de suppression des branches/propositions.
- Aucun test fonctionnel/HSM/cloud/production exécuté ou qualifié par cette ratification ; aucune autre ADR Accepted modifiée.