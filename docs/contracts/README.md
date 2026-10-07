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
