# Correctif intégré et revu ; passage aux tests d'intégration

L'objectif reste de rétablir la connexion après inscription sans relâcher les
frontières de sécurité. Le correctif et la sélection ciblée des tests sont
maintenant intégrés localement, après deux revues sécurité sans défaut bloquant.
Les sept empreintes de sources revues sont conformes ; aucun push applicatif.

Les 19 tests du domaine, trois tests RS256, compilation IAM et formatage restent
des preuves valides réutilisées. La compilation du binaire d'intégration ne vaut
pas exécution : ses neuf cas vont maintenant tourner nativement sous Windows,
avec PostgreSQL réel fourni par les fixtures existantes et le nettoyage tracé.

Le laboratoire utilise encore l'ancienne application. L'image IAM corrigée et le
vrai navigateur viendront après le résultat des tests et du nettoyage. Aucune
réussite globale n'est annoncée.

## Références techniques

- Commit local7b8c1f95faa7c5db8d7aa9bcfbfb37fd8cb56ccd : neuf chemins explicitement
  autorisés, hook normal réussi ; aucun changement étranger embarqué.
- Revue sécurité complémentaire1538Z : aucun finding ; lifecycle/observeur hors
  resolver strictement inchangés. Briefing1533Z sur les cinq sources IAM conservé.
- Préflight : slot Cargo libre,24jobs/E: sans override, neuf tests source ; daemon
  déjà actif, inventaire contemporain six conteneurs dont trois nœuds possédés et
  ressources préexistantes non adoptées. L'inventaire complet est repris par le
  lanceur avant création, pas déduit des anciens comptes.
- Commande : `powershell.exe -NoProfile -File ops/tests/it-native.ps1 -Suite iam -TestBinary oauth_browser_transactions`,
  avec RUN_ENV=test dans le seul processus de campagne. Threads par défaut conservés.
