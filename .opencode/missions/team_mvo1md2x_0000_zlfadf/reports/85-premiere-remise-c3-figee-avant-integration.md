# Rapport 85 — première remise C-3/RunningApp figée

## Provenance recoupée

SDK tests **3333ee0a480c4b38081039550768c3e50bbd8a4b**, parent9e37149, treea46cdf6b4408899aa017ddc60f8ac39759f438eb, clean. Production avant cfg(test) comparée à9e : identique. Source entière finale SHA256bda058b9e14d203e81235eb52257dda7ada3015a660ce00864cd8d9501c8947c.

Lead vérifie chaque hash des25artefacts auteur, archive avec quatre sources et manifeste (30entrées) sous `.tmp/http-compat-c3-delivery-evidence/manifest.json`. BundleSDKtests SHAec6557546ae048097ca288fb8bc2cd4f3fbacf19091ae2bf54e1dbed9a71be66.

AI tentative normale auteur unique bloquée hook206 ; lead contrôle index/scope, puis checkoutcourt `.worktrees/hc3`, commit **c9566a3b943274990ee638a6dbd557d553adeb44** normal hookPASS, tree3569ccbedef15da58029beeba8a08c24b4caea07 exact. Trois fichiers tests/fixture +31/−3. Branche auteur actualisée CAS après revalidation sans reset/stash/bypass. Pas gitlinkfinal committé ; aliasQA privé préservé.

## Preuves finales, distinctes des intermédiaires

Refactor helperrequest test-only pour lint, puis C3 rejoué sur source finale : `--lib http::owned_server::tests::owned_http2_announces_and_routes_extended_connect -- --exact --nocapture` exit101/1FAILED/0ignored/45filtered. SETTINGSfalse/H2probevrai/CONNECT non reçu, owner+driver nettoyés avantassert. Les rouges intermédiaires sont conservés, pas confondus avec cette preuve finale.

FmtSDKglobal0 et Clippy nonstrict0 avec warnings existants (aucun strictPASS/Sonar nouveau). `cargo test --locked -p telegraph-service --test outbox_runtime_tests --test notification_created_routing_test --no-run` exit0, deux exécutables compilés. **Aucune IT exécutée.** DiffAI lu par lead : parents0/2 plus URL liée au owner conservé et healthHTTP200 borné2s, assertions métier/replay/FGA/stop non réduites.

## Barrière

Shutdown natif pool12 demandé avant team_merge ; première remise pas encore intégrée au lead. Task reste en cours car C-2 attend le helper production réel. Aucun autre runtime/DB/Docker créé ; bailCargo auteurreleased0process et hooklead terminé. Conserver même session pour C-2 après entréehelper intégrée, sans prétendre écrire le seam par auteurtests.

Après intégration, production pourra corriger C-3 et extraire le startup commun **sans normaliser encore le port** ; cela fournit l'entrée du témoin C-2 avant son fix. Pas nouveau backend/API SDK/feature/deps/harness. Publication/pin toujours bloqués, C-2/C-3 ouverts.
