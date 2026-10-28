# Deux chemins de reprise observés, mais la cause de l'échec reste ouverte

## Ce que l'analyse permet de conclure

La mission vise une notification durable, récupérable après panne et accessible uniquement aux bonnes personnes. Le dernier diagnostic réussit ce scénario complet, mais la validation précédente avait échoué après reconstruction. Le même expert a terminé leur comparaison en lecture seule, avec effort natif **high confirmé avant analyse**.

Les deux exécutions ne sont pas équivalentes au moment de l'arrêt :

- Dans l'exécution rouge109, l'ancien dispatcher trouve une intention à reprendre environ20ms avant la demande d'arrêt. Sa mise à jour de prise en charge est loggée pendant l'arrêt ; son résultat réellement enregistré n'est pas connu.
- Dans le diagnostic vert110, aucun nouveau claim de l'ancien dispatcher n'est observé. Après l'arrêt, la ligne est toujours failed, sans verrou. La nouvelle instance la reprend et la publie. Ce succès ne vient pas d'une longue pause ayant laissé expirer un verrou de30s : seulement environ138ms séparent début d'arrêt et fin de reconstruction.

Le contraste situe donc une fenêtre d'ordonnancement très courte autour de l'arrêt. **Il ne prouve pas que le verrou explique historiquement109.** Les états absents ne sont pas reconstruits à partir des logs. La mesure de109ms dans110 inclut le poll de100ms ; ce n'est pas une latence exacte d'envoi ni un engagement de service.

## Risque et garanties à ne pas confondre

Le risque observé est que la publication n'atteint pas toujours le contrôle de reprise de25s. Un état publishing peut légalement rester verrouillé30s après une annulation, tant que le nombre d'essais le permet. Le contrat prévoit déjà une reprise après expiration du verrou ; il ne promet pas un SLA produit de25s.

Cela ne démontre ni perte de notification, ni défaut d'identité ou de droits, ni tâche locale orpheline : une commande SQL distante déjà émise peut survivre à l'annulation de sa future locale. Mais un seul vert instrumenté ne ferme pas le rouge109. Publication, pin final et acceptation globale restent bloqués ; aucune garantie n'est abaissée et aucun délai modifié.

## Vérification de faisabilité complémentaire du lead

Sans exécuter de test, le lead a vérifié les surfaces déjà présentes : `OutboxDispatcher::new` accepte déjà un `EventPublisher` injecté ; le test `shutdown_during_publish_destroys_future_and_preserves_unpublished_same_id_for_new_dispatcher` possède déjà un publisher de test suspendu à son entrée, de vraies fixturesSQL/SQS et la reconstruction du dispatcher. Ce test historique utilise un verrou de100ms, pas le verrou par défaut de30s.

Un témoin contrôlé peut donc réutiliser ces surfaces plutôt que demander un nouveau hook de production ou une infrastructure. Il faut cependant distinguer son point d'annulation : le test historique force une tâche suspendue après la grâce d'arrêt, alors que le wrapper réel observé peut annuler le dispatcher immédiatement via son signal. La nouvelle preuve doit expliciter cette frontière ; ne pas recopier aveuglément le test historique et annoncer qu'il reproduit109.

## Choix recommandé et frontière d'autorisation

L'unique diagnostic110 est consommé. Répéter simplement le scénario vertical pourrait produire un nouveau vert sans expliquer le rouge : l'expert le déconseille.

**Recommandation : autoriser un seul test de composant contrôlé, avec les fixtures et interfaces existantes.** Il placerait délibérément une intention réellement prise en charge enSQL dans le publisher de test suspendu, annulerait son propriétaire à la frontière explicite, puis reconstruirait sur la même DB. Il mesurerait état/identité/tentatives/verrou et comportement dans les25s avec le verrou par défaut de30s. Aucune mise à jourSQL synthétique pour fabriquer l'état, aucun sommeil aléatoire pour gagner une course, aucun hook ou correctifSDK/production.

Ce test serait une **nouvelle preuve diagnostique**, pas un remplacement de la verticale ni une baisse de ses assertions. S'il montre une absence de reprise tant que le verrou est actif, cela établira prospectivement ce mécanisme dans la situation contrôlée, pas les valeurs historiques manquantes de109. Il n'autorise ni hausse du délai existant, ni validation après expiration hors budget, ni correctif ou publication automatique. Préparation/inspection puis une seule exécution, nettoyage tracé ; durée exacte non garantie.

**Autre choix : conserver le blocage.** Aucun nouveau test ni changement. Les corrections et le succès110 restent conservés et documentés, mais le risque de reprise demeure non résolu et la publication bloquée. Aucun risque accepté implicitement.

## Annexe de preuve

- Expert : même pool16, tâche `task_mvqhraw4_000r_68zqce`, `team_handoff({phase:'implementation',reasoning:'high'})` accepté ; aucune édition, Cargo, SQLruntime ou ressource.
- Diagnostic110 sourceSHA5f02f067 ; logSHA `095721ecd229194cdaba68aa1fa32b3e2a17a2ce7e77d277dfa5ff8791e0b35a`. Ligne995préstop,996/1000stop,1002projection,1018/1019rebuild/wait,1023nouveauworker,1026/1027claim,1030marquage,1032published ;1062résultat.
- Rouge109 : l987dernier candidat vide08:43:51.521994 ; l992candidat53.528318 ; l995stop53.548660 ; l997update53.549343. Les0rows loggés ne prouvent ni commit ni absence de commit.
- Vert110 : ancien candidat vide09:34:39.386690 ; stop41.387031 ; captures après arrêtfailed1/locksNULL. Prochain tour nominal2s proche de l'arrêt, sans prétendre connaître l'heure effective de réveil du scheduler.
- Faisabilité source : SDKdispatcher::new72–83 ; trait EventPublisher dans rustycog-events/src/event.rs38–49 ; TI/services/Telegraph/tests/outbox_runtime_tests.rs293–399, SuspendedPublisher et témoin existant avec lock100ms/poll20ms. Aucun test exécuté pour cette vérification.
- Localisation : claim/annulation/reconstruction, non une modification du métier. Impact : prochain témoin défini à partir de surfaces existantes. Limites : causalité historique109 inconnue, instrumentation non neutre, aucun taux de panne estimé, aucun nouveau run actuellement autorisé. Cleanup110 historique indépendant :0possédé actif/Cargo0,12préexistants préservés ; ne vaut pas nouvel inventaire.
