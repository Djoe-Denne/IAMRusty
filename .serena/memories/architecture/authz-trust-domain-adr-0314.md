# Autorité du principal jusqu’à OpenFGA — ADR 0314

Statut: Accepted. Réalité: Unimplemented. Ratification explicite lead le 2026-10-28 sous délégation utilisateur de la mission authentification révision 1 ; aucun test fonctionnel déduit de cette ratification.

- Canon: docs/adr/0314-principal-authority-through-openfga.md ; contrat docs/contracts/authz-trust-domain-boundary.md. Voir le fichier ADR.
- Principal exact (iss, sub) dans lecteurs, cache et projections ; plateforme historique seulement avec provenance/politique explicites, autres autorités encodées dans le type user existant.
- Pas de fallback UUID, alias par suffixe, double grant permissif ou bannissement global des identités organisation ; refus local aux frontières UUID-only et positifs publics préservés.
- Hive conserve l’issuer du membre/propriétaire cible dans events/write/delete ; transition coordonnée et fermée sur état legacy ambigu.
- 0305 reste Accepted/Implemented sur son périmètre AuthN ; aucun supersede ni preuve de code livré. Commit documentaire auteur d159c7b intégré, métadonnées ratifiées par le lead ; implémentation/tests encore à faire.
- Hook fmt documentaire OS206: échec environnement, pas ROUGE fonctionnel ; livraison docs-only autorisée avec exception Git éphémère, contrôle Rust intégré non dispensé.