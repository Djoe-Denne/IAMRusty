# Contrôle d'intégration final de Manifesto

Le correctif Manifesto passe le formatage, les cinq tests unitaires ciblés et la
compilation de son consommateur monolithique. La revue sécurité indépendante ne
relève aucun défaut démontré dans ce delta. Cela ne suffit pas encore à prouver
son fonctionnement avec une vraie base de données.

Je réutilise donc une campagne native du service Manifesto avec le runner existant.
Le constructeur et les configurations partagés affectent ses fixtures ; cette
campagne inclut les deux suites de droits et consentements prévues. IAM et Hive,
déjà vérifiés, ne seront pas relancés, et aucun nouveau harness n'est ajouté.

Les dépendances de test seront créées par les fixtures existantes. Leur arrêt et
nettoyage normal seront contrôlés par leurs identifiants exacts, y compris en cas
d'échec. Les ressources préexistantes, dont les nœuds Kind, ne seront pas adoptées
ou arrêtées par cette campagne. Le code testé est Windows, pas un service dans Kind.

Le correctif Envoy passe aussi sa régression discriminante et les 12 tests source.
Sa revue de delta continue ; son démarrage réel, le mauvais certificat et le
parcours navigateur restent à démontrer après les vérifications d'intégration.

## Commande et preuves attendues

- Commande réelle : ops/tests/it-native.ps1 -Suite manifesto.
- Le runner utilise cargo test -p manifesto-service --tests ; pas de revendication
  erronée du flag --locked. Lockfile et sources gelés par empreintes avant/après.
- Un seul Cargo natif, 24 jobs, cible E:, témoin Docker et registres contemporains.
- Compteurs de tests et inventaire final attendus ; aucun résultat encore déclaré.
