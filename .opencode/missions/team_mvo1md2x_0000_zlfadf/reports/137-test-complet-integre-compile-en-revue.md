# 137. Test complet intégré et compilé ; revue ciblée engagée

Le candidat reçu a été inspecté puis appliqué au seul test vertical par le lead.
La source intégrée est identique octet pour octet au candidat. Aucun fichier de
production, SDK, fixture partagée ou configuration n'a été modifié.

Le changement borne la reconstruction et la réception du vrai événement depuis
l'éligibilité stockée, en conservant explicitement les handles et l'owner HTTP
pour le cleanup. Le parcours de droits et replay reste à vérifier par le reviewer
puis par l'unique exécution autorisée ; aucun PASS comportemental n'est déduit.

## Vérifications réellement exécutées

- `git diff --check` : succès.
- `rustfmt --edition 2021 --config skip_children=true --check services/Telegraph/tests/notification_created_routing_test.rs` :0.
- `cargo test --locked -p telegraph-service --test notification_created_routing_test --no-run` :0,30,76s.
- SlotCargo Windows/jobs24/targetE respecté et libéré ; aucun processusCargo
  restant au contrôle. SDK propre, sources figées pour la revue.

La compilation n'exécute aucun test et ne démarre aucune fixture. L'autorisation
de la validation verticale unique n'est donc pas consommée.

## Revue indépendante

Brief persisté avant admission, contexte fresh lecture seule, sources/diff/contrat
et faits vérifiés sans conclusions auteur. Le périmètre porte sur la deadline,
les await tardifs, la possession/jointure des tâches et les oracles conservés.
Pas de nouvelle chaîne QA, scan global ou infrastructure. Des défauts bloquants
seront renvoyés au même auteur avant validation ; pas de relance à l'aveugle.

Annexe : sourceSHA80cce027ef5eefb41d6a5f457847a94feccebec7ecf28941fbb9fec5338211a7,
delta6ac7+284/-18 ; logs/summary et source sous TI/.tmp/vertical-final134-inputs.
Brief `.cursor/review-briefings/20261027-vertical134-deadline-and-ownership.md`,
copie brief-review134.md SHAb6a45bb0 ; tâche revue task_mvr206hz_000m_k62qlp.
Localisation : preuve verticale Telegraph. Impact : changement test-only compilé.
Limites : ni validation verticale, ni publication/pin, ni fermeture Clippy/Sonar.
