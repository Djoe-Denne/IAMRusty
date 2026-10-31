# Bootstrap arrêté avant création : correction Windows ciblée

Le bootstrap s'est arrêté avant de créer le moindre nœud. Les 20 conteneurs
préexistants sont inchangés et le témoin de créations a été arrêté. Seul un
répertoire d'ancrage vide a été créé ; les preuves de cet essai sont conservées.

La frontière fautive est localisée : les helpers de protection des fichiers
passent leur chemin comme argument après une commande PowerShell. Sous Windows,
cette forme ne lie pas cet argument à `$args` comme attendu. Une reproduction
inoffensive confirme le problème, sans toucher aux droits ni créer de cluster.

Je corrige uniquement le passage des paramètres, sans élargir les droits, changer
les règles de confidentialité ou désactiver la garde. Les chemins seront traités
comme données, et les erreurs resteront bloquantes. Des tests ciblés précèdent
une nouvelle tentative avec un bail neuf ; aucune reprise forcée de l'ancien.

Les dix images déjà vérifiées restent disponibles. Ce blocage de bootstrap
n'invalide pas les IT ou les compilations réussies ; Envoy et navigateur restent
non exécutés. Aucun nouvel arbitrage utilisateur n'est nécessaire dans le mandat.

## Preuves

- Essai : `sh_185900b410012ZJb0plTHoVTFE`, bootstrap exit 1, zéro création de nœud/volume.
- Bail pending 263d86bb13a5452f88bf8bf88be50303, sans UID ni kubeconfig.
- Reproduction native : invocation actuelle exit 1 / argument absent ; bloc de script témoin exit 0 / argument présent.
- Localisation : `ops/deploy/local_full_target.py`, helpers `secure` et `private_file` ; consommateurs bootstrap et validation des entrées OAuth.

## Correctif vérifié, revue avant reprise

Les trois tests ciblés Windows et les 24 tests de la suite passent maintenant.
La validation native a aussi trouvé un ordre incorrect dans le scellement :
le fichier perdait le droit de modifier ses attributs avant de devenir read-only.
L'ordre est corrigé, sans ajouter de droit final.

Deux arrangements intermédiaires de test ont échoué et restent archivés ; ils
ne sont pas présentés comme des succès. Quelques marqueurs non secrets scellés
par ces essais sont conservés plutôt que forcer leur réouverture. Aucun runtime
ne tourne pour ces tests. Le diff est soumis à une revue de sécurité ciblée
avant le prochain bootstrap ; les images déjà vérifiées n'ont pas changé.

Pendant la revue, les certificats locaux de campagne ont été préparés avec
les outils Windows existants, sans Docker ni WSL : neuf certificats vérifiés,
deux autorités distinctes. Les entrées OAuth neuves sont privées et read-only ;
leur scellement et leur contrôle réels ont réussi avec le correctif. Aucune
clé ou valeur d'authentification n'a été imprimée. Ce n'est toujours pas une
preuve de handshake Envoy ou de navigation.

La revue sécurité est maintenant reçue : aucun défaut sur les postimages exactes.
Le correctif est intégré et le contexte propre actualisé. Un nouveau bail privé
est créé ; l'ancien essai n'est pas repris. Les images conservent leur identité
de compilation et le bootstrap utilise son nouveau commit de conduite.
