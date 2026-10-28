# Rapport 72 — HTTP RP-1 et RP-2 réellement rouges

## Exécutions recoupées par le lead

Logs/JSON lus via Context Mode dans pool10 `.tmp/http-c1-e2-rp-evidence/`. Deux commandes SDKlib ciblées, compilation réussie, chacune exit101 /0PASS/1FAILED/0ignored/44filtered :
- filtre `owned_t1_expired_deadline` : premier join **Err(HTTP coordinator join unobserved at final deadline)**, deuxième et troisième **Ok(forced=true)** ; RAII0 au premier,1 après rejoin, mutation0, cleanuptrue. Échec de l'assertion sticky à owned_server.rs892, après récolte réelle.
- filtre `owned_t2_force_progresses` : futur join annulé à10ms ; pas de destruction observée sans repoll jusqu'à deadline+30ms ; RAII0 avant libération, mutation1 après ; join repris **Ok(forced=false)**, cleanuptrue,94ms. Assertion destruction indépendante ligne955 rouge.

Lecture source des tests836–963 recoupe vraie socket/entrée handler, budgetT1passé et runtime current-thread, T2grâce30ms/deadline60ms et observation de la notification RAII, pas de sleep supposant l'entrée. Observations capturées puis handler libéré, RAII/socket et owner nettoyés avant assertions rouges. Aucun fix production ni seam ajouté par l'auteur tests.

Ces résultats confirment les deux mécanismes du briefing1901Z ; ils ne réouvrent pas le RP-1 SQS historique. Les16+10verts initiaux restent valides dans leur portée, mais ne ferment pas ces deux défauts. Zéro nouveau runtime Docker.

## Suite autorisée et barrière

Auteur E2 finalise commits tests-only et provenance, sans rerun ou correction production. Intégrer la première passe et ces témoins avant auteur correctif. Correctif production à borner dans l'owner existant : diagnostic de deadline durable distinct de la complétude, force pilotée par mécanisme privé possédé indépendant du futur join et du coordinateur ; tous handles/timers eux-mêmes possédés/joints, pas watchdog détaché. Pas d'API/backend/harness nouveau ou contrat réduit.

Publication/pin final et clôture C-1 restent bloqués. Après correction, même oracles et tests ciblés, revue indépendante reprise ; les commandes déjà acquises ne sont pas rejouées par principe sans impact. Témoin rouge n'est pas une campagne de correction ratée : il apporte un progrès vérifiable et une cause confirmée.
