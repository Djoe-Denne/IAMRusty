# Test contrôlé autorisé, sans modifier le produit

## Décision

Après l'analyse116, l'utilisateur choisit **« Autoriser le test contrôlé »** : préparation inspectée puis une seule exécution d'un test de composant réutilisant les fixtures et interfaces existantes. Vraie prise en charge SQL, publication suspendue, annulation du propriétaire, reconstruction sur la même DB, observation de25s et verrou par défaut de30s. Aucun changement de production, de délai ou d'assertion de la verticale ; aucun correctif, pin ou publication automatique.

## Nature de la preuve

Ce témoin place volontairement le composant après un claim réellement terminé, puis interrompt la publication. Il ne simule pas un état publishing par UPDATE et ne tente pas de gagner une course par sommeil aléatoire. Il teste prospectivement un mécanisme possible ; il ne mesure pas rétroactivement le claim interrompu pendantSQL109.

L'oracle diagnostique peut constater qu'un verrou encore actif empêche la reprise dans la fenêtre observée. Un résultat conforme à cette prédiction n'est ni un PASS de la verticale rouge ni une preuve de perte. Les états, identités, tentatives, propriétaire/date du verrou et échéance doivent être corrélés. Si la fenêtre ne permet pas de départager les états, signaler le diagnostic inconclusif, pas un résultat causal inventé.

## Routage et étapes

Même test-engineer pool14, continuation native sansfresh avec high autorisé à confirmer avant édition. Plan QA intégré avant admission : `.cursor/qa-plans/20261027-controlled-claim-cancellation117-once.md`. Scope : ajout local à `services/Telegraph/tests/outbox_runtime_tests.rs`, interfaces `OutboxDispatcher::new`/`EventPublisher` et fixtures existantes. Le publisher suspendu et l'ancien test restent inchangés autant que possible. Pas de hookSDK, nouvelle dépendance ou nouveau harness d'exécution.

L'annulation doit être explicitée : le témoin historique attend la grâce puis force une tâche bloquée avec un verrou100ms. Le nouveau témoin doit exercer une annulation possédée après claim, sans confondre ce point avec le watch-stop réel ou l'annulation pendant SQL109. Réutiliser les mécanismes d'ownership et de stop existants ; aucun détachement, abort aveugle ou budgetétendu.

Auteur : préparation/fmt/compilation de la cible avec --no-run seulement, pas de runtime/test. Lead : inspection du diff et des oracles, archivage/hash/intégration de la preuve test-only si retenue, puis préflight/bail frais et **une seule exécution** du témoin. Aucune autre verticale, NoOp, suite globale ou répétition jusqu'au vert. Toute preuve d'envoi après expiration demanderait un budget distinct : elle n'est pas autorisée tacitement ici.

SDKd60 local/index129, AC1–7 et statuts ADR inchangés. La correction temporelle106 reste dans le produit ; instrumentation110 temporaire isolée préservée. Anciennes preuves97/105/109/110 immuables. Ressources antérieures non adoptées ; créations du nouveau run tracées/cleanup même échec, zéro possédé actif à sa fin.

## Admission effective de préparation

Tâche `task_mvqloafu_000v_i4yt9h` publiée sans contextefresh puis réaffectée nativement au même pool14, branche et arbre conservés ; état working vérifié une fois. Six entrées de plan/autorisation/analyse/sourcebaseline/trait/politique ont été copiées et leurs empreintes vérifiées avant publication. Baseline du fichier composant `008a97375a497ee599bd7297384e2a4d70b28d13f1564237461b2c5f59be5f23`, identique entre lead et auteur. Confirmation d'admission transmise avant demande de high/édition ; résultat du réglage et préparation attendus. Aucun runtime ou nouveau test encore lancé.
