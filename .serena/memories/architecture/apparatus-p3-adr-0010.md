# ADR-0010 — gate pré-prod certificat / CA workload

- Jalon : P3 (complète 0007 ; pré-production après P3 Implemented)
- Canon : `docs/adr/0010-gate-preprod-workload-certificate-ca.md`
- Statut : Proposed
- Réalité : Unimplemented
- SuperSède : aucune

- Limite CA logicielle (`platform-internal-ca`) / CSR vs keypair ouvert / certificat quelconque = OK hors prod (laptop/kind), **bloquante avant prod**
- Exigences avant prod : modalité d’émission + autorité CA adressées ; interdit de présenter 0007 Implemented comme prêt prod sur ce point
- Mécanisme (CSR-from-workload / keypair injecté / produit CA / PKI / TTL) = Non décidé
- Ne SuperSède pas 0007 ; pointeur « ne pas shipper » dans 0007 Non décidé ; sœur 0009
- Voir le fichier ADR
