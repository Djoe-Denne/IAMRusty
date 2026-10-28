# Rapport 43 — AC6 nominal prouvé, phase après panne encore bloquée

## Preuves recoupées

Run T3 `t3-ac6-063b92d1c2bb4ce2` : summary.json et extraits du log lus via Context Mode. Cible routing exit101,6PASS/1FAILED/0ignored,38.23s : cinq classifiers et témoin SMTP passent, test vertical global reste échoué.

Marqueur réel du chemin nominal : event_id1e1c47ed-8ef0-4ec7-844b-59a04d484d9c, notification_id43661b09-4de4-438e-9ffa-a02a9f942d5a, published, tuple_count1, HTTP403 autre utilisateur, HTTP200 destinataire, is_readtrue, publisher_outagefalse. L'auteur rapporte que le replay même bytes/ledger neuf est passé ; le lead a vérifié que ce replay demeure dans le source avant le marqueur (routing415–420). Ce progrès ferme nominalement le défaut de classification précédent, sans valider toute AC6/AC2 ni la revue indépendante.

Échec ultérieur : stored_notification attend25s puis panic ligne262 pour la phase après panne, avant reprise publisher. Source315–342 : suppression de la seule file sortante, nouvelle instance runtime, nouvel événement IAM envoyé, puis attente notification. Ce n'est pas le même échec que le400 de replay Sentinel ; ne pas le reclassifier en collision SMTP ou en succès global.

## Diagnostic borné

Les extraits du log existant n'exposent pas de cause consumer/SQL utile. Hypothèses à distinguer, pas conclusions : sortie prématurée d'un handle supervisé ; entrée non reçue/traitée ; échec de transaction avant notification. Autoriser instrumentation test-only dans l'ownership routing/fixture existant : observer résultat/erreur des vrais handles pendant l'attente, corréler l'ID entrant et sa file, diagnostics SQL/consumer nécessaires sans secrets. Ne pas augmenter le timeout ou masquer l'erreur, ni changer la production par intuition.

Un run diagnostic ciblé du seul test vertical peut suivre un préflight frais et nouveau bail, sans rejouer les cinq classifiers ou les suites vertes. Si cela révèle une cause, corriger dans le périmètre existant ou remonter une frontière explicite. Tenir le registre des hypothèses : ce run a apporté un progrès vérifiable nominal ; le nouveau blocage après panne n'a pas encore deux tentatives techniques infructueuses, pas d'escalade emergency automatique à ce stade.

## Ressources

Summary :4nouvellesfixtures, owned_running[], avant5/après6 ; node40201...running et quatre SMTP historiques intacts, nouveau SMTP86062cd283bbae93f85dd54a052676acb289e2c0357184466697ce981e6ad3cd exited retenu ask annoncé ; deux nouveaux volumes opaques ask. Pas d'opération Docker lead nouvelle pour ce recoupement. Bail du batch terminé, inventaire contemporain et bail à renouveler avant diagnostic ; aucune suppression des retenus.

Préflight diagnostic annoncé ensuite : six containers full-ID = node et cinq SMTP arrêtés historiques, aucune fixture possédée running ; douze volumes opaques ask à préserver. Auteur a ajouté uniquement select sur wait_for_background_failure production pendant l'attente, logger existant INFO sans exporter Loki et corrélations IDs bornées ; fmt seul annoncé exit0. Bail renouvelé/GO explicite transmis pour UN run diagnostic. Demande de filtrer le seul test vertical dans les args Cargo (guards wrapper inchangés, hash/delta archivés), plutôt que rejouer les six tests unités déjà acquis. Aucune nouvelle exécution lead ; résultat diagnostic encore attendu.
