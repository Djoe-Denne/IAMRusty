# ADR-0009 — gate pré-prod scale on demand

- Jalon : P4 (complète 0008 ; pré-production après P4-core)
- Canon : `docs/adr/0009-gate-preprod-scale-on-demand-isolation-instance.md`
- Statut : Proposed
- Réalité : Unimplemented

- Limite pod-H24 / partage-par-digest = OK hors prod (laptop/kind), **bloquante avant prod**
- Exigences avant prod : scale on demand + pas de partage d’instance inter-projets silencieux
- Mécanisme V1 = ADR-0011 (pod-par-binding) ; `Réalité : Implemented` de 0009 ssi 0011 est Accepted **et** Implemented
- Ne SuperSède pas 0003 ni 0008 ; pointeur « ne pas shipper » dans 0008 Non décidé
- Voir le fichier ADR