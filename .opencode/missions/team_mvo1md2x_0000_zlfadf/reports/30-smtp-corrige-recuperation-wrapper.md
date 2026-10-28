# Rapport 30 — correction SMTP annoncée, réutilisation du wrapper T2

## Livraison locale annoncée

T3 rapporte suppression complète du fallback SMTP stop/rm-f par nom. TestSmtp conserve l'Arc OwnedTestContainer ; cleanup refuse un bail actif en conservant le singleton, agit sur le seul handle possédé et propage les erreurs. Mécanismes existants réutilisés : fixture_runtime.rs222–253/718–820 (collision fail-closed, Arc lease, stop/rm ID), database.rs85–105 pour DB possédée.

Témoin unitaire annoncé réellement exécuté : `common::fixtures::smtp::testcontainer::ownership_tests::smtp_unknown_and_active_consumers_are_preserved_without_cleanup`, 1PASS/0ignored/1filtered. Log `.tmp/t3-evidence/smtp-ownership-unit.log` du worktree T3 ; quatre cibles IT recompilées no-run annoncées PASS. Le lead n'a pas encore recoupé diff/log. Aucune IT ni opération Docker annoncée.

## Continuité de preuve, pas nouveau harness

T3 demande la copie locale du wrapper opérationnel T2 pour registre/inventaires/cleanup des quatre cibles, plutôt qu'une campagne large ou un nouveau harness. Le lead a vérifié le dossier `.tmp` T2 et ses deux dossiers de runs : ils contiennent les preuves, pas de script wrapper persistant. La session T2 d'origine est sollicitée une fois pour retrouver le script réellement exécuté dans son historique, sans Cargo/runtime/édition ni nouvelle tâche. Ne pas présenter une reconstruction comme l'original.

En parallèle, T3 peut préparer diff et préflight en lecture seule. Le run IT reste bloqué ; aucune autorisation implicite de modifier it-native ou d'étendre l'infrastructure de preuve. Si l'original n'est pas récupérable, borner explicitement la solution avant exécution. Pas de test/compilation redondante du lead et pas de PASSglobal.
