# Envoy démarre ; livraison ciblée de Manifesto

La mission doit encore prouver le refus d'un mauvais certificat et le parcours
d'authentification dans un vrai navigateur. Le blocage de démarrage d'Envoy est
levé : son nouveau Pod est prêt avec la même image et les mêmes protections de
transport. Seule l'expression rejetée a été remplacée par les six routes équivalentes.

Les images Linux validées sont disponibles. Je livre maintenant uniquement
Manifesto : nouvelle image vérifiée et nouvelle configuration explicitant son
autorité de service. L'ancienne configuration et les neuf autres images sont
conservées ; aucun secret, job ou stockage n'est réinitialisé.

Les 15 tests d'intégration ciblés réussis restent les preuves natives pertinentes.
La disponibilité d'Envoy n'est pas encore la preuve de son contrôle du nom dans
un certificat, ni une réussite du parcours navigateur. Ces vérifications suivent
la reprise de Manifesto et le contrôle infra ciblé.

## Annexe technique

- Source636b438, SDK02e5 ; Envoy1.31.2 inchangé.
- Reprise Envoy exit0/ready, UID ConfigMap et Deployment conservés.
- Pod Envoy6d2960ba… remplacé par cec77212-9d63-42e7-925a-3c1a771367fb.
- Preuve privée : envoy-routes-recovery-cb1350af10d640499e5a8b473aa59390/summary.json.
- Image Manifesto19ccec47… reliée à l'artefactfe567047…, Linux/amd64, appuser.
- Pas de nouvelle compilation ou de validation globale implicite.
