# Rapport 75 — continuité native refusée, substitution explicitée

## Admission réelle

Correctif publié sous `task_mvpnqoc6_0027_vk1lrv`, après E2 et témoins réellement intégrés. Tentative native de fork depuis auteurpool9 vers `http-rp-fix`, sans worktree supplémentaire, refusée explicitement : **Non-transferable source/worktree changes: commit or resolve them before inheriting into another directory**. Aucune promesse d'héritage réussi et aucun fallback fresh silencieux.

État diagnostic unique après refus : auto-dispatch a affecté **pool-11, hard-implementer**, worktree isolé ensemble-04d9a2f8-zlfadf-pool-11, branche ensemble-bfc5cd57-d8f9-4dd8-909e-e99f411ed58d. C'est une nouvelle session fresh, pas une continuation native de l'historique pool9. Aucun override modèle/classification/politique. Mandat publié contient une gate explicite source/ownership/entrées avant toute mutation.

## Décision de coordination dans le périmètre approuvé

Préserver l'arbre auteur, ses commits SDK locaux et artefacts ; ne pas committer un pin non publié, stash/reset ou modifier des changements pour satisfaire le garde-fou. Continuer dans le worktree isolé déjà affecté avec **handoff écrit et vérifié** (rapport74/planpool9, contrat, revue, deux rouges, SHAs exacts) ; signaler cette substitution plutôt que prétendre hériter des conversations. Aucun coût/scope/garantie ou modèle utilisateur supplémentaire n'est demandé, pas de réparation du harness.

pool9 demeure source du plan validé et n'est pas autorisé à coder en parallèle. Pool11 ne reçoit GO code qu'après confirmation native racine/source SDK/ownership et copies locales. Si l'accès exact manque, aucun contournement par sourcelead ou anciennephotographie T4 : matérialiser les vraies entrées dans le seul worktree autorisé.

Le correctif demeure SDK privé owned_server.rs au-dessus426dc24, tests intacts, contrats/API/protocoles inchangés, pas push/pin final. C-1 reste ouvert ; aucun nouveau test ou productionfix acquis par cette décision d'affectation.
