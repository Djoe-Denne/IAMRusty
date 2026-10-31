# Correctif reçu : contrôle positif à aligner avant application

L'objectif reste de rétablir l'entrée du navigateur sans élargir les contrôles
d'authentification. Le candidat est reçu : une seule adresse de tunnel vers le
port d'Envoy, avec vérifications d'identité, modification conditionnelle et retrait
limité à sa propre règle.

Ses quatre tests simulés réussissent. La lecture du candidat a cependant révélé
une erreur dans son contrôle de réussite : il attendait une redirection HTTP302,
alors que le parcours existant exige303. Cette erreur aurait refusé une réponse
légitime et retiré la règle. La correction et un test discriminant sont demandés
au même auteur, avant intégration ou application.

Aucune règle réseau n'est encore changée. La revue sécurité indépendante, le
contrôle réel de l'entrée, puis les preuves Envoy et navigateur restent à faire.
La conduite prévue est inchangée ; aucune décision utilisateur n'est nécessaire.

## Traçabilité

- Candidat isolé : deux fichiers autorisés seulement ; quatre tests PASS selon
  l'auteur, contrôle de périmètre et empreintes recoupés par le lead.
- Rapport final récupéré dans la session native après notification sans livrable.
- Détail : `.cursor/review-briefings/20261029-envoy-ingress-parent-witness-status.md`.
- Enveloppe parent préparée, non exécutée et verrouillée sur une livraison finale
  encore inconnue ; aucune image applicative à reconstruire pour ce correctif.
