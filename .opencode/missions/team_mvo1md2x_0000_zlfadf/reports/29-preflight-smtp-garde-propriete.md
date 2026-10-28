# Rapport 29 — préflight SMTP : nettoyage non sûr bloqué

## Constat annoncé par l'auteur

Avant toute IT/opération Docker, t3-runtime signale dans `services/Telegraph/tests/fixtures/smtp/testcontainer.rs` : lignes106–110 appel cleanup_container, lignes159–181 stop puis rm -f par nom `telegraph_test-smtp`, sans preuve de propriété. Les cibles historiques HTTP/résilience/routage passent par common et pourraient l'exécuter. Constat source fourni par auteur, pas encore recoupé directement par lead ; aucune suppression ni création runtime annoncée.

## Décision technique dans le périmètre approuvé

Maintenir l'interdiction de run tant que ce chemin est atteignable. Les adaptations bornées `tests/fixtures/**` sont déjà dans l'ownership T3 : autoriser la suppression du fallback par nom et la réutilisation exclusive des mécanismes OwnedTestContainer existants, sur IDs effectivement enregistrés et sans bail actif. Cette autorisation de code ne vaut pas autorisation de retirer un conteneur existant ni de contourner les garde-fous permanents.

Pas de nouveau harness/backend, nouveau système global de baux, adoption d'une ressource par son nom ou suppression forcée. Une ressource inconnue ou en cours d'utilisation reste intacte ; un conflit doit échouer explicitement ou utiliser un mécanisme existant à ports/noms isolés. Si l'API Owned existante ne permet pas ce changement local, remonter la frontière avant extension.

Exiger un témoin borné sans Docker si les abstractions existantes le permettent, le diff et la vérification des effets transitifs avant nouveau bail IT. Pas d'affaiblissement de l'oracle SMTP ni de désactivation des cibles historiques pour obtenir un succès.

## Autres résultats annoncés et limites

Deux tests monolithe passent après correction test-only de fixture IAM, zéro ignoré ; logs à recouper lors de remise. Clippy strict reste NONVERT : deux dead_code préexistants annoncés dans configuration/lib.rs:220/223, aucun masque. Ce sont les premiers diagnostics rencontrés, pas preuve qu'il n'en existe aucun autre. Les quatre diagnostics framework antérieurs restent distincts. Aucun PASSglobal ni IT exécutée.
