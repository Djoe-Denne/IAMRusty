# SDK : revue indépendante terminée et commit exact publié

## Résultat et décision

Le SDK conserve l'autorité de l'utilisateur jusqu'aux permissions. Sa modification a passé les vérifications ciblées et une revue de sécurité indépendante ; aucun blocage n'a été identifié dans ce diff. Le lead a publié le commit exact sur une branche de mission du dépôt canonique, conformément à la préautorisation utilisateur.

La publication a réussi et l'identifiant distant a été contrôlé égal au commit autorisé. La branche de mission a été créée sans force-push. Le gitlink applicatif n'est pas encore sélectionné dans le canon ; la livraison applicative reste en cours et ne doit pas être intégrée incomplète.

Cette publication rendra la dépendance récupérable ; elle ne clôture pas la correction applicative. La collision de droits dans Hive, les projections et les migrations restent à vérifier avec leurs tests intégrés.

## Preuves et limites

- Snapshot SDK propre, parent initial vérifié, onze fichiers dans le périmètre. Aucun changement étranger du checkout SDK canonique embarqué.
- Traces natives Windows recoupées : check réussi ; suites permissions et HTTP réussies, soit 53 tests distincts, aucun ignoré. Les sous-runs superposés ne sont pas additionnés. Formatage finalement réussi, distinct de son premier échec de format.
- Revue fresh du snapshot immuable : passes défensive et adversariale, aucun finding. Issuer exact dans sujets/caches, encodage borné, refus UUID seul, purge des extensions héritées, absence de repli anonyme après refus. Aucun test relancé par le reviewer. Rapport persistant relu par le lead.
- Remote canonique vérifié sans credential dans son URL ; branche de mission absente au préflight. Diff whitespace propre, un seul nouveau commit depuis le parent. Aucun force-push, réécriture ou fusion distante autorisé.
- Envoy et les contrats navigateur ratifiés sont intégrés. Les fichiers livrés ont été comparés par identité de contenu aux commits des auteurs. Le consommateur laboratoire est publié sur ces entrées réelles.
- Slot Cargo SDK restitué et désormais réservé au signataire distant. Aucun Docker, Kind ou WSL de mission créé/redémarré. Preuves réelles Envoy/navigateur et revue globale après IT encore absentes ; aucun PASS global ou production.

## Incident d'enregistrement

La première écriture de ce rapport a été refusée par le contrôle natif edit, puis une lecture a été annulée avec « location shut down ». Les catalogues se sont rechargés. Aucun autre outil n'a servi à contourner l'écriture refusée et aucun droit n'a été modifié.

Diagnostic en lecture seule : journal d'incident recoupé ; profil orchestrator du projet et permission native globale allow confirmés via API explicitement rattachée au projet ; aucune demande de permission en attente dans la session. La cause exacte du refus n'est pas établie. Une seule nouvelle tentative par le même outil patch a été choisie après ce changement d'état observable. Un nouveau refus aurait laissé l'écriture bloquée, sans contournement.

## Annexe technique

- Commit préparé : `72845d9ddfc590852602791eb50cfad6262d80bc`, parent `d60a88b2f2f499768a807941a1b4c80c9a366b3f`.
- Destination prévue : `github.com/Djoe-Denne/rustycog`, `refs/heads/mission/authz-principal-sdk-pool3-20261028`.
- Revue : `msg_183f47310001OCG7TXbEmmAXyP`, `.cursor/review-briefings/20261028T1918Z-security-sdk-autorite-72845d9.md`.
- Tests recoupés : `msg_183eaad8b001XkTvJPUCgoDSY5`, `msg_183eb98b6001byIT3KR5t7Q2I5`, `msg_183ebabc7001m4qWRUtpfNmRyy` ; check `msg_183ea5deb0017tVlHJvQLfcYTx`.
- Intégration Envoy et navigateur : `435376f297e2068255d589752e215258a170a595` ; consommateur laboratoire `task_mvsi2ar1_001j_93xlbp`.
- L'API de configuration sans paramètre location utilisait le dossier personnel, pas le projet. Le diagnostic utile a employé le paramètre location[directory] documenté, sans changer de session/profil/configuration. Ce détail n'affecte pas les lectures de sessions par identifiant.
