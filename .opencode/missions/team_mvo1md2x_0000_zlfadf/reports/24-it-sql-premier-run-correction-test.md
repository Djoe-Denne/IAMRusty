# Rapport 24 — IT SQL : 7PASS, scénario fautes à rejouer

## Résultats recoupés

Run t2-outbox-94bc985f70a24c01, source/logs du worktree T2 sous .tmp/ :
cargo.log : running8tests,7passed/1failed/0ignored/0filteredout,8.30s ; exit101
dans summary.json. Le test real_delivery_and_outbox_sql_faults_rollback_the_entire_attempt_then_recover
échoue à l'installation de faute SQL, PostgreSQL42P01 relation outbox_events
inexistante. La table partagée est rustycog_outbox_events. Ce n'est pas une
preuve rouge de production : l'oracle fautes outbox n'a pas encore été exercé.

Le membre corrige uniquement le test via EntityName, pas de production changée,
ni d'affaiblissement de l'oracle rollback0/0/0+infrastructure. Rejouer uniquement
ce scénario et conserver les sept preuves valides du premier run.

## Ressources et preuve de fin du premier bail

Registre .tmp/docker-resources-20261025.log lu : container possédé
aad7b9d98ff26866d895814533363964e554132bb1a97aa6d10c43e61b309424
créé, stoppé, reclassifié yes puis retiré Exited0 sans force/suppression volume.
summary.json : owned_running[], before_count1/after_count1, issues[]. Le seul
remaining est40201bda8bb2d1eeaa51287be96fc3448752c219dba74791fa326c5071705f29
running, préexistant/non possédé, préservé.

Volume anonyme observé, création opaque : idUNKNOWN,
Name29d7354c0713da89eb516799e62681282c46a568a96b54797bfe5e504538400b,
safe_to_delete ask, conservé. Image en cache non adoptée. Pas de cleanupglobal.

## Suite

Bail renouvelé pour le seul scénario ciblé et ses nouvelles fixtures possédées,
registre exact-ID/cleanup mêmeéchec. Attendre1test réellement exécuté et vert,
pas de rerun global ni présentation du premier batch comme8/8PASS. Aucun Cargo
ou runtime supplémentaire lancé par le lead/reviewer.
