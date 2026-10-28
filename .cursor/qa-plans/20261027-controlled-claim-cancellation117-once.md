# Témoin117 unique — claim terminé puis annulation possédée

Autorité : choix utilisateur«Autoriser le test contrôlé» après116, contrat117. Base AI1f2/SDKd60 propre/pin129, test vertical produit6ac7 ; fichiers temporaires110 restent isolés et ne sont pas exécutés par ce témoin. Aucun nouveau contrat produit/garantie, ADR0313/0503Proposed et0301Partial, AC1–7 inchangés.

## Ownership et surfaces autorisées

Même pool14/test-engineer après admissionnative, efforthigh dans son enveloppe. Seul `services/Telegraph/tests/outbox_runtime_tests.rs` modifiable. Réutiliser TelegraphTestFixture/DB/SQS, notification/service/row, OutboxDispatcher::new et EventPublisher injecté. Le SuspendedPublisher existant et son entrée Notify fournissent déjà un point après claim réellement terminé ; pas de hook SDK/prod/dépendance/feature/harness nouveau. Conserver les tests existants et leurs assertions.

## Préconditions et oracle contrôlé

1. Créer une notification/outbox par le vrai service sur la fixtureSQL existante ; retenir l'identité/payload comme oracle sans les imprimer en clair. Publisher test-only suspendu à son entrée, après le vrai claimSQL achevé. OutboxConfig par défaut : lock30s,poll2s,backoff5s,max20 ; ne pas reproduire le verrou100ms du témoin historique.
2. Attendre une synchronisation possédée et bornée de l'entréepublisher, constater dansSQL publishing/identité/attempts/locked_by/locked_until. Aucun UPDATE fabriquant cet état, transactionheld ou FORUPDATE d'observation.
3. Annuler le propriétaire via un signal possédé/une frontière explicite, puis jointure sous le mécanisme existant et budgetstop6s. Prouver la destruction du publisher suspendu/propriétaire. Pas de tâche détachée. Le point d'annulation choisi doit être documenté : après claimcommitté n'est pas pendantSQL109 ; ne pas prétendre reproduire l'application entière si wrapperlocal.
4. Reconstruire le dispatcher sur la même DB avec transport libéré/réel existant. Vérifier que la fenêtre d'observation25s est réellement contenue dans le verrou mesuré ; sinon diagnostic non discriminant à signaler, pas d'allongement ou manipulation d'horloge. Deadline25s unique calculée au point annoncé, sans reset, pas d'attente après expiration30s implicite.
5. Montrer activité du nouveau dispatcher/sélection réelle et comportement durable pendant la fenêtre. Corréler mêmeid/event/aggregate, état/tentatives/locks/dateéchéance. Si aucun claim n'est admissible avant deadline sous verrou, l'absence de publication peut être le résultat attendu du TÉMOIN, pas une réussite de la garantie verticale25s. Ne pas changer ou masquer le rouge109. Si reprise plus tôt ou état différent, en garder les observations et conclure suivant faits, pas forcer l'hypothèse.
6. Arrêter/join tous propriétaires sous budgets existants même en échec ; pas de nouvelle campagne pour valider après expiration. Ne pas augmenter délais pour rendre le témoin vert. Préserver assertions identité/persistance/transport pertinentes ; aucun résultat de droits/replay non exécuté n'est revendiqué.

## Sécurité des traces et périmètre

Champs sûrs seulement : IDs,état,tentatives,next_at/locked_by/locked_until/updated_at, présence booléenne d'erreur, horloges UTC/monotones et compteur de cycles. Paspayload/JWT/secret/DebugModel/erreur brute. Reprise et SQL réels, queue opt-in via fixtures existantes ; aucun Kind ou application conteneurisée.

## Gate auteur puis gate lead

Préparation seule : lire sources actuelles/plan et confirmer high après admission ; ne pas refaire enquête116. Fmt du seul fichier et `cargo test --locked -p telegraph-service --test outbox_runtime_tests --no-run`, Windows24unique après préflight/bail. Aucun test/Docker/SQLruntime pendant préparation.

Livrer diff depuis sourceoutbox_runtime actuelleAI1f2,sourceSHA,assertions etnom exact du nouveau filtre, protocole/limites etmanifestecodes. Préserver tousfichiers utilisateur/staged/SDK/temporaire110. Lead inspecte puis archives/intègre et vérifie sourcehash, launcher existant adapté à un SEUL filtre de composant, préflight/inventaire/bail frais, UNE exécution autorisée. L'ancien run105/106/110 n'est pas une permission de retry.

Nouveauharness/hookSDK ou besoin de budget après expiration = STOP et arbitrage avant mise en œuvre. Aucune production/pub/pin/acceptationglobale. Cleanup par identités fraîches, préexistants etASK préservés, aucun arrêt global Docker/WSL.
