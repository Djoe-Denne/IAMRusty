# Rapport 55 — C-1 E1 reproduit avant correction

## Résultat recoupé

Lead lit via Context Mode les fichiers réels `.tmp/http-c1-e1/{command.json,result.json,cargo.log}` du worktree pool-8. Commande exécutée : `cargo test --locked -p telegraph-service --test outbox_runtime_tests http_shutdown_destroys_inflight_handler_before_return_without_late_mutation -- --exact --nocapture --test-threads=1`.

Base aa400f38608e92a34c463f1f89b02168f8364440, SDK129dcc1575e7dd59b4c7935de0b3794dbb7e7d07. Exécution du 2026-10-26T17:45:41Z au17:46:24Z ; exit101, 0PASS/1FAILED/0ignored/4filtered, test0.51s. Compilation réussie puis assertion C-1 à la ligne180 : ce n'est pas une panne de préflight.

Log787 : shutdown=Ok, elapsed_ms=0, destroyed_at_return=0, mutations_at_return=0 ; après libération/nettoyage destroyed_after_cleanup=1, mutations_after_cleanup=1, parent_destroyed=1, response_bytes=83. Le parent est terminé mais le handler survit et peut muter après le retour du vrai helper d'arrêt.

## Source et portée

Lecture ciblée du test réel non mergé : socket TCP loopback, entrée effective synchronisée, route async pending avec propriétaire RAII, vrai `rustycog::http::serve_router`, vrai `shutdown_owned_tasks`. Le select-stop reproduit la frontière des callers standalone/host ; ce témoin ne démarre pas toute l'app ou le monolithe. Libération du handler, RAII, EOF et socket.shutdown précèdent l'assertion rouge ; cleanup borné. Aucun Docker ni ressource plateforme.

Diff actuellement stagé strictement un fichier existant `services/Telegraph/tests/outbox_runtime_tests.rs`, +158lignes ; aucun diff unstaged observé. Contrôle diff unstaged propre, contrôle index/commit final encore à remettre par auteur. Aucun fix production, pin ou manifeste modifié. Source du worktree non indexée canoniquement : lecture du fichier réellement exécuté, pas conclusion depuis l'index partagé.

Une première lecture Context Mode du lead a échoué sur l'échappement d'une expression régulière ; corrigée avec séparation explicite par codepoint, puis lecture réussie. Pas de panne MCP ni fallback.

## Suite

Auteur prépare commit normal et provenance, sans rerun ; slot Cargo E1 libéré selon son retour, aucun Cargo lead lancé. E1 est une preuve rouge acquise, pas une correction ou une clôture AC7. Attendre puis intégrer le témoin et le contrat architecte avant publication du consommateur SDK ; T5 reste préservée et T4 C-1 ouvert.
