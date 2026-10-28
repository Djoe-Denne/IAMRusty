# Rapport 34 — runtime vert, routage bloqué par collision SMTP

## Preuves recoupées

Run auteur `t3-runtime-a14848fdbcbe476b`, sous `.tmp` du worktree T3. Lead a lu summary.json, résultats des deux logs Cargo, opérations ciblées et lignes du registre pour le SMTP retenu.

- outbox_runtime_tests : exit0,4PASS/0failed/0ignored,32.85s. Noms exécutés : témoin SMTP ; disabled/degradedNoOp+HTTP+rebuild ; SQS long-poll shutdown/destruction ; annulation publish/reprise mêmeID.
- routing : exit101,1PASS(témoin SMTP)/1FAILED,0ignored. Parcours vertical échoue en setup notification_created_routing_test.rs90 : foreign/unknown fixture name or published-port collision; STOP. Ce n'est pas une preuve rouge du comportement de production.
- HTTP et résilience : non exécutées, explicitement listées par summary.
- Neuf IDs de fixtures tracés, owned_running[], unknown_new_ids[], avant1/après2 : node40201...running préservé et SMTP26b3...exited. Quatre volumes opaques ask conservés.

## Cause et conservation

Auteur rapporte SMTP `26b3eaa589e9861a4786960c15dd22962adc4381f4a5a63c1de2d17e9c19e3ce`, nom fixe telegraph_test-smtp, Exited2 ; wrapper retire seulement Exited0. Le nom retenu bloque donc la création du binaire suivant, qui refuse correctement toute adoption/éviction par nom. Le lead n'exécute pas de Docker inspect supplémentaire pour ce constat. Le registre montre la provenance créée par la mission ; ce conteneur ne devient pas « non possédé » parce qu'il est préexistant au prochain batch.

Conserver cet ID arrêté. Demander une reclassification append-only ask pour rétention/diagnostic et absence d'autorisation de retrait Exited2. Aucun rm2, force, suppression par nom, stop node, suppression volume ou modification de la règle Exited0.

## Décision locale et reprise ciblée

Autoriser nom SMTP suffixé UUID par création via with_container_name/start_service_testcontainer existants, même valeur transmise aux deux API, ports/configuration/oracles inchangés. Vérifier que les validations de nom existantes l'acceptent sans modification framework. Pas nouveau harness ni nouveau backend ; sinon remonter la frontière.

Réduire uniquement la liste targets du wrapper inspecté aux trois cibles routing/HTTP/résilience, sans refaire les quatre succès runtime pour cette modification de nom. Archiver le delta source et les hashes avant reprise. Bail renouvelé pour les seules nouvelles fixtures après inventaire frais excluant node et SMTP26b3...retenu. Les autres gardes du wrapper restent inchangées ; erreur/issue arrête les cibles restantes avec cleanup et preuves absentes explicites. Cette reprise demeure dans les cibles approuvées, pas une nouvelle campagne globale. La preuve verticale réelle reste absente.
