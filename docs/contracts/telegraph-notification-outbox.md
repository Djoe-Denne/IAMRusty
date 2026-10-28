# Contrat — création et publication durable des notifications Telegraph

- État décisionnel : **Proposed**, ratification humaine distincte ; implémentation autorisée et livrée, sans Accept implicite (réconciliation2026-10-28).
- Date : 2026-10-25 ; jalon : fiabilité Telegraph (hors P0–P6 Apparatus).
- Base examinée : `93e376c5530a439aaafe61a64d3e881865c1acf1` (Git du worktree vérifié par le lead).
- Canon : [0301](../adr/0301-outbox-transactionnel-rustycog.md), [0300](../adr/0300-crates-events-contrat-sans-transport.md), [0102](../adr/0102-setup-composition-root.md), [0200](../adr/0200-it-infra-reelle-rustycog-testing.md), [0201](../adr/0201-mocks-http-sortant-seulement.md), [0202](../adr/0202-transport-opt-in-queues-desactivees.md), [0303](../adr/0303-sentinel-sync-worker-fga.md).
- Décision complémentaire : [ADR-0313 Proposed / Implemented](../adr/0313-telegraph-notification-outbox-atomique.md), matérialisée avec son digest. 0301 reste Accepted / Partial pour l'écart IAM ; ses mentions Telegraph sans outbox sont historiques.

État de livraison : application39024b2 et SDKd60a88b intégrés respectivement à
AIForAll master et RustyCog main ; voir le [rapport150](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/150-mission-et-sdk-integres-aux-branches-principales.md).
Les sections de préparation et gates ci-dessous décrivent le déroulé historique
et les exigences conservées, pas des permissions restant à obtenir pour une
implémentation déjà livrée. Les preuves finales et limites restent celles du
[bilan141](../../.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/141-livraison-terminee-avec-reserves-explicites.md) ; la publication initialement locale décrite par ce bilan est dépassée par le rapport150.

## 1. Décision et frontières

Une création réussie enregistre notification, livraison éventuelle et une intention `NotificationCreated` dans **une seule transaction writer PostgreSQL**. La réponse métier dépend de ce commit, jamais de la disponibilité du publisher. Le dispatcher partagé publie ensuite l'intention durable, au moins une fois sous réserve de reprise et du budget d'essais partagé.

Le choix de frontière est de **renforcer le port existant `NotificationWriteRepository`**, pas d'ajouter un second port d'enregistrement autonome ou une transaction application supplémentaire. Les signatures publiques des deux créations et leur résultat métier restent identiques. `CombinedNotificationRepositoryImpl` délègue sans ouvrir ni terminer de transaction. Aucun type SeaORM, pool ou dispatcher n'entre dans le domaine.

Le stockage est obligatoire même avec la queue désactivée. Disabled/no-op suspend uniquement la publication ; il ne supprime ni ne valide fictivement les intentions. Le schéma `telegraph-events`, les routes HTTP et le translator Sentinel ne changent pas.

### Périmètre et propriété

- **T2**, sous-lot préalable autorisé : cycle de vie cancellation-safe de `rustycog-events::SqsEventConsumer`, tests ciblés et intégration du pin publié (§5.2), puis documentation du port domaine ; service de notification ; writer et délégation infra ; migration ; dépendances strictement nécessaires ; injection minimale dans setup ; nouvelle cible `services/Telegraph/tests/outbox_tests.rs`.
- **T3** : publisher/dispatcher dans setup ; supervision standalone/monolithe ; raccordement test-only Sentinel ; `outbox_runtime_tests.rs` ; tests nominaux/negatifs impactés ; transfert explicite de setup et Cargo.lock depuis T2.
- **Lead** : validation/intégration T1, ADR/digest/pointeur, ledger commun et arbitrages. T2 fresh après contrat intégré, aucun fork T1→T2.
- Hors scope : framework/pin rustycog hors sous-lot cancellation-safe explicitement autorisé (§5.2), nouveau backend ou service, déduplication IAM entrante, réparation des notifications historiques, rétention/rejeu opérateur nouveau, garantie exactly-once, scaling/fencing du poller, modification OpenFGA ou du contrat d'événement, Docker applicatif/Kind.

## 2. Interfaces et algorithme atomique (T2)

### 2.1 Contrat consommé

Les signatures suivantes restent celles de `NotificationService` et du port writer :

```text
create_notification(NotificationCommunication)
  -> Result<NotificationCommunication, DomainError>
create_notification_with_delivery(NotificationCommunication, CommunicationMode)
  -> Result<(NotificationCommunication, MessageDelivery), DomainError>
```

Nouveau sens obligatoire : chaque succès comprend exactement une notification et une ligne outbox correspondante ; le second comprend aussi exactement une livraison, le premier aucune. Aucun constructeur de production ne conserve une création sans outbox. `create_delivery` seul ne produit pas un nouvel événement de création de notification ; lecture, mark-read et nettoyage restent hors de cette transaction de création.

`NotificationWriteRepositoryImpl` reçoit le writer et `OutboxRecorder` (constructeur explicite recommandé : `new(writer, recorder)`). Une aide privée commune peut vivre dans `notification_write.rs` ou `infra/src/transaction.rs`, mais **pas deux implémentations divergentes de la transaction**. `NotificationServiceImpl::new` ne reçoit plus de publisher : il reçoit le repository. Supprimer `publish_notification_created` et les deux appels réseau post-persistance ; ne pas conserver un fallback best-effort.

Le repository concret est l'adapter du port atomique : il construit le contrat `telegraph-events` à partir des valeurs métier normalisées et appelle le recorder partagé. Un port générique `record_event` qui ouvre sa propre transaction est interdit (l'écart IAM documenté en 0301 n'est pas un modèle à copier).

### 2.2 Préparation et transaction

1. Avant toute écriture, valider `recipient.user_id` et les champs requis par les mappers existants. Absence de destinataire = validation (`InvalidRecipient`), non un échec réseau post-commit.
2. Choisir une fois le `notification_id` (préserver l'ID fourni, sinon UUID neuf) et le `created_at` (préserver celui fourni, sinon une valeur UTC unique). Fixer les valeurs par défaut persistées avant de bâtir l'événement ; pas de deuxième `now()` de substitution après commit.
3. Construire une fois `NotificationCreatedEvent::new(notification_id, user_id, created_at)` dans `TelegraphDomainEvent::NotificationCreated`. Son `event_id` est distinct de l'ID SQL de la ligne outbox. Conserver l'ID de l'événement généré pour toute sa vie durable.
4. Ouvrir la transaction sur le **writer** ; ne jamais utiliser la connexion de lecture pour une écriture ou une vérification participant au commit.
5. Insérer la notification via cette transaction. Pour la seconde opération, créer la livraison avec `message_id = notification_id`, puis l'insérer via **la même** transaction.
6. Réaliser les mappings fallibles du résultat et la validation de cohérence IDs/timestamps avant commit. Appeler **une seule fois** `OutboxRecorder::record(&txn, &event)` : sa sérialisation/métadonnées/version et l'insert doivent réussir dans cette transaction.
7. Commit une seule fois ; retourner uniquement les résultats déjà validés. Aucun `publish`, nouveau mapping fallible ou fabrication d'ID requis après commit.

Toute erreur préalable au commit entraîne rollback des écritures de la tentative, y compris notification déjà insérée et livraison. Journaliser séparément un échec de rollback sans remplacer silencieusement l'erreur initiale. Le recorder peut sérialiser pendant la transaction : le point obligatoire est **avant commit**, pas une sérialisation après retour métier.

Payload inchangé : enum sérialisé `{event_type: "notification_created", data: ...}` avec les vrais `notification_id`, `user_id`, `created_at` et les champs BaseEvent. `aggregate_id = notification_id`. Aucun ID reconstruit par le dispatcher.

### 2.3 Erreurs et limites de tentative

- Begin/insert SQL/record outbox/commit : conserver une catégorie infrastructure (`DomainError::InfrastructureError` puis `ServiceError::infrastructure`), avec contexte d'opération ; pas `EventProcessingError`/Business ni Internal arbitraire pour un incident DB. Le domaine expose déjà `is_recoverable()` pour infrastructure.
- Une sérialisation du recorder en erreur fait échouer la transaction et ne devient pas un succès avec événement absent. Une erreur intrinsèque de données reste distinguée d'un incident transport ; pas de boucle de retry infinie ajoutée.
- Après commit, une erreur de publication est du ressort du dispatcher et **ne remonte pas** comme échec de création au consumer IAM. Elle ne déclenche donc pas à elle seule une seconde création.
- `[command.retry] max_attempts=0` reste « pas de retries ». Il est distinct du budget `OutboxConfig.max_attempts`.
- Un résultat de commit ambigu (connexion perdue lors du commit) ne permet pas de promettre zéro ligne ni une relance exactement une fois. Signaler cette limite ; aucune déduplication des commandes ou messages IAM n'est introduite.

## 3. Migration et reprise (T2)

Ajouter une **nouvelle migration Telegraph** après `m20250201_000001_create_notification_tables`, enregistrée dans `migration/src/lib.rs`. Ne pas réécrire l'initiale déjà appliquée. Le `up` délègue à `rustycog::outbox::outbox_migration().up(manager)` et le `down` à son symétrique. Réutiliser schéma, statuts et index partagés, sans copier une variante SQL comme la migration Hive historique.

La dépendance rustycog nécessaire appartient à `telegraph-migration` ; réutiliser la version/features déjà résolues ; seul le changement de pin cancellation-safe autorisé au §5.2 est permis. Le harness `Migrator::up/down` doit exécuter aussi cette migration. Preuves : DB vide → up complet ; DB avec ancienne migration et notification existante → up sans perte ; up répété sans seconde table ; down du seul ajout sur DB de test sans supprimer les notifications.

Déployer la migration **avant** le binaire qui écrit l'outbox. Une table absente est une panne de persistance, pas une autorisation à publier directement. Un rollback de schéma supprimerait des intentions non publiées : jamais une procédure de livraison automatique ; down testé uniquement sur DB possédée. Aucun backfill historique.

Le dispatcher utilise `DbConnectionPool`, le publisher signé existant et `OutboxConfig`. Les statuts observés sont ceux du framework : `pending`, `publishing` sous verrou, `failed` avec `next_attempt_at`, puis `published` après retour `Ok` d'un **vrai** transport. Une panne laisse `failed` ou un claim `publishing` récupérable après expiration, sans fabriquer de nouvelle ligne métier.

La reprise crée une **nouvelle instance** sur la même DB, sans mémoire du premier dispatcher ; elle republie l'événement stocké avec le même `event_id`. Le poller partagé limite les essais (défaut observé : 20), ce n'est pas une garantie de livraison après une panne arbitrairement longue. L'épuisement reste visible et durable ; ne pas annoncer une reprise automatique illimitée ni ajouter un mécanisme opérateur dans ce lot.

Le claim partagé observé n'établit pas un contrat de fencing multi-worker. Pour cette tranche, un seul dispatcher actif par DB Telegraph (standalone **ou** monolithe, pas les deux sur le même writer). Les doublons de publication restent possibles entre envoi et marquage published ; l'aval doit converger.

## 4. Disabled, dégradation et routage (T3)

`setup` conserve les informations de `SignaledPublisher` : `publisher`, `status` et `transport`, déjà produits par `readiness::create_signaled_multi_queue_event_publisher`.

- `ComponentStatus::Live` **et** transport concret non-NoOp : autoriser le dispatcher. « Live » prouve la construction du client, pas la disponibilité du broker ; une panne réseau reste un échec de publication durablement retryable selon le budget partagé.
- `Disabled` ou `Degraded` (factory tombée sur NoOp) : ne pas démarrer le dispatcher, ne pas appeler `dispatch_once`, ne pas claim les lignes. Conserver pending/failed, tentatives, IDs et payload. Signaler la suspension par les signaux readiness/logs existants ; garder HTTP en vie.
- Un `Ok(())` de NoOp ne prouve **jamais** published. Ne pas se fonder seulement sur `queue.enabled`, le succès d'initialisation ou un health-check NoOp. `Injected` reste test-only et ne constitue pas une preuve de transport réel AC5.
- Reprise après disabled/degraded : reconstruire l'application avec un transport réel, puis drainer la même table. Aucun hot reload/recréation automatique de client n'est promis. Le test prouve cette reconstruction lorsque nécessaire.
- Vérifier que `notification_created` résout une destination sortante non vide. Conserver la séparation `inbound_queue_config` : le consumer Telegraph ne consomme jamais sa file AuthZ sortante. En test, la destination actuelle est `test-telegraph-outbound`, pas un nom déduit du handbook. Vérifier aussi l'absence de message sur le fallback/inbound pour cette publication.

## 5. Supervision et budget QA (T3 — gate explicite)

### 5.1 Contrat de handles

`start_background_tasks` reste compatible avec la surface monolithe : `Vec<JoinHandle<anyhow::Result<()>>>`. Retourner **tous** les handles démarrés, consumer actif et dispatcher actif ; zéro handle est valide si les deux transports sont suspendus. Le standalone ne doit plus faire `pop()` puis ignorer le reste. Ne pas filtrer un handle avec `is_finished()` avant d'avoir consommé son résultat : une erreur déjà terminée doit être observée.

Le propriétaire du runtime surveille tous les résultats, y compris panic/JoinError. Une sortie attendue disabled/no-op ne tue pas HTTP ; une sortie inattendue d'une tâche active est un échec supervisé. Le dispatcher partagé journalise ses erreurs de cycle et conserve les erreurs de publication en DB : les tests ne doivent pas attendre un Err de `start()` pour chaque panne réseau. Les erreurs de démarrage, panics et arrêts sont contextualisées et rendues, pas seulement perdues dans un handle détaché.

Déclencher l'arrêt du consumer et du dispatcher, puis attendre **tous** les handles possédés. En monolithe, inclure ces handles dans la supervision et le drain existants ; ne pas appeler `TelegraphApp::run()`. L'erreur d'origine et les erreurs de cleanup restent observables. Le cleanup HTTP possède son groupe de descendants, pas seulement le handle serveur : admission, annulation et jointure suivent le [contrat HTTP C-1](http-runtime-owned-shutdown.md), §5.3 ci-dessous, sans tâche orpheline après le retour ordonné.

### 5.2 Annulation sûre : sous-lot framework préalable T2

**Arbitrage utilisateur explicite relayé par le lead le 2026-10-25 : option 1 autorisée**, correction minimale cancellation-safe dans rustycog-events + tests ciblés + intégration du pin. AC7 intégral est conservé ; aucune réduction à l'arrêt coopératif. Cette autorisation de périmètre ne vaut pas Accept autonome de l'ADR complémentaire.

Défaut confirmé statiquement par le lead sur le pin `6c63b65c23e26b8f83f1c6f63d656ef31c7591d0`, sans diff local sqs.rs : `SqsEventConsumer::start` spawn les pollers et garde leurs handles dans un Vec local, `stop` ne fait que lever un flag. Annuler le parent ou retourner sur le premier enfant en erreur abandonne les autres handles. Le drain monolithe cinq secondes puis abort expose cette fuite. Aucun test n'est déclaré exécuté sur ce constat.

**Propriétaire : hard-implementer T2**, sous-lot préalable à l'atomicité Telegraph, sans tâche autonome supplémentaire. Surface autorisée : `rustycog-events/src/sqs.rs`, tests ciblés de son cycle de vie et modifications locales strictement nécessaires à cet ownership. Pas de refonte transports, nouveau backend/harness, nouveau protocole ou API générale d'events. Préserver la surface publique `EventConsumer::{start,stop}` et le comportement de livraison/retry existant hors annulation ; une impossibilité exige un retour au lead, pas une extension silencieuse.

Contrat de réalisation :

1. Le parent `start` possède tous ses pollers dans une structure cancellation-safe (recommandation : `tokio::task::JoinSet`, jamais des handles oubliés lors du Drop). Aucun spawn non enregistré.
2. Arrêt demandé : signal partagé par instance, vérifié aussi autour des awaits du long-poll et du handler, pour interrompre une attente async en cours. Ne pas dépendre d'un futur prochain poll d'un AtomicBool après un handler bloqué. Conserver le reset de cycle de vie sans perdre un stop concurrent au démarrage ; interdire/encadrer un double start sur la même instance.
3. Erreur/panique d'un enfant : observer le premier résultat, signaler/annuler les frères, attendre leur fin, rendre une erreur contextualisée. Ne pas attendre séquentiellement indéfiniment un autre poller avant de découvrir cette erreur. Les erreurs de polling actuellement retryées/loguées ne deviennent pas toutes des arrêts fatals ; les erreurs terminales/panics ne sont pas cachées.
4. Retour normal ou en erreur : tous les enfants sont joints. Annulation/drop du futur parent : le propriétaire détruit/annule tous les enfants ; les tests attendent des témoins de destruction effective, pas uniquement la fin du parent. Le runtime propriétaire normal attend son `start` et tous ses propres handles avant retour.
5. Cancellation pendant traitement : ne pas acquitter/supprimer le message comme traité si le handler n'a pas réussi. Une transaction locale annulée doit se rollback ; un commit déjà réussi peut conduire à redelivery. La correction n'introduit pas de déduplication IAM ni exactly-once.

**Budget QA proposé : 6 secondes au total**, deadline absolue unique depuis stop/annulation jusqu'à terminaison/destruction observée de toutes les tâches Telegraph possédées : grâce jusqu'à 5 secondes, puis au plus 1 seconde pour annulation async et join/observation des enfants. Ce n'est ni une durée par handle ni un SLA de production. Le runtime doit inclure dans ce budget les appels stop, dispatcher et HTTP possédés, pas démarrer le chrono après des stops potentiellement bloquants. Alignement possible avec la grâce monolithe actuelle de cinq secondes, mais son abort parent ne suffit qu'après livraison de l'ownership cancellation-safe. Un dépassement est une preuve rouge, jamais un succès par abandon des handles.

Portée exacte : tâches async Tokio annulables et runtime qui continue à les exécuter. Ni arrêt garanti de code synchrone non coopératif/boucle CPU sans yield, ni terminaison de `spawn_blocking`, ni kill de processus/OS ou terminaison d'effets réseau distants. Les tests de blocage utilisent un handler **async** en attente annulable ; ne pas annoncer une garantie physique plus forte. Un publisher suspendu ne doit pas permettre published ; un publish annulé peut avoir été accepté à distance et sera rejoué avec le même ID.

Tests framework ciblés obligatoires : arrêt normal ; stop pendant long-poll ; handler async bloqué ; annulation/drop du parent ; erreur terminale et panique enfant avec un frère actif. Dans chaque cas, témoin RAII de destruction/fin de **chaque** enfant et absence d'activité après arrêt ; erreurs observées et aucun message acquitté avant succès. Réutiliser l'infra/tests existants ; helpers/futures de test locaux autorisés pour forcer les entrelacements, pas un nouveau harness. Le long-poll réel est recoupé en T3 sur la fixture SQS existante. Archiver les noms/commandes réels et rouge/correction/vert, puis T3 réutilise cette preuve avec AC7 standalone/monolithe.

Intégration selon le skill `rustycog-submodule` : inventaire Git/propriété avant toute mutation du dépôt framework ; travail isolé dans le dépôt source rustycog autorisé par le lead, sans checkout/stash/reset/commit des changements utilisateur préexistants. Commit du seul correctif possédé, publication du SHA sur le remote avant intégration du gitlink AIForAll (mode `160000`). Épingler exactement ce SHA revu, pas `--remote` vers un HEAD mouvant ; préserver `[patch.crates-io]` vers `rustycog`, sans copie vendor ni patch vers un sibling. Le lead coordonne accès, intégration et pin ; aucune publication/pin n'est exécutée par T1. Le lot atomicité et T3 consomment le SHA réellement intégré. Un accès externe refusé reste un blocage, pas une autorisation d'écrire ailleurs.

### 5.3 Extension HTTP SDK autorisée — C-1

L'utilisateur a choisi **Autoriser HTTP SDK**, rapport53 : API additive bornée sur pile existante, tests ciblés, publication/pin exact et adoption Telegraph standalone/monolithe. Cette autorisation est distincte du sous-lot SQS §5.2 et de l'exception Sentinel §6.1 ; pas nouveau backend/harness, refonte, baisse d'AC7 ou Accept implicite.

Contrat à intégrer avant correction : [propriétaire HTTP et arrêt AC7](http-runtime-owned-shutdown.md), référencé par [ADR0503 Proposed / Unimplemented](../adr/0503-http-runtime-owned-shutdown.md). Le caller conserve le propriétaire ; sortie serveur et jointure exhaustive sont distinctes. Connexions, requêtes, bodies/streams, exécuteur HTTP/2 et handshakes pertinents entrent dans le budget commun six secondes stop inclus. Drop ordonne l'annulation sans promettre un async join ; graceful illimité et parent joint seuls ne suffisent pas. Préserver serve_router legacy, TLS/mTLS/dual-bind et protocoles.

**E1 rouge réel est acquis**, baseline historique aa400f3/SDK129dcc1, puis intégré dans b291eee3b7537bfa02cebb5df844d5b4ca2019bf ; blob221abe3fc7a1b751c8a1d574b422f767ee68abca, rapports55/57. 0PASS/1FAILED/0ignored/4filtered : shutdownOk et parent terminé, RAII handler0 au retour, mutation tardive1 après libération ; cleanup avant assertion. Aucun SDKfix livré, C-1 reste ouvert. L'entrée E1 est intégrée ; l'implémentation reste bloquée jusqu'à intégration du contrat/API. Pas besoin de relancer E1 pour écrire ce contrat.

Références dépôt principal : `.cursor/review-briefings/20261026T1704Z-correctness-telegraph-outbox-t4-aa400f3.md` et `.cursor/qa-plans/20261026-http-ac7-c1.md`. QA ciblée via cibles existantes, loopback sans Docker ; préserver les deux docs T5 staged ailleurs. AC1–AC7, exclusions exactes §5.2, §6.1/§6.2, ADR0313 Proposed et0301 Partial restent inchangés.

## 6. Raccordement test-only à Sentinel (T3)

Ne démarrer ni le binaire Sentinel ni toute la plateforme. Ajouter si nécessaire **uniquement une dev-dependency** de `telegraph-service` vers `workers/sentinel-sync` (crate `sentinel-sync`) et `telegraph-events`. Sa lib expose déjà `handler`, `translator`, `idempotency`, `fga_client`. Aucun changement production Sentinel hors exception étroite autorisée au §6.1 ; ce raccordement ne nécessite aucun changement framework supplémentaire au sous-lot §5.2.

Construire dans la fixture de test le vrai `SyncEventHandler::new` avec : `TelegraphTranslator::new()`, `InMemoryEventLedger::new()` derrière `Arc<dyn EventLedger>`, et `OpenFgaWriteClient::new(openfga.client_config())`. La fixture expose le client config du **même** store/modèle OpenFGA réel que le checker HTTP. Pas de tuple préchargé, `allow`, `allow_all`, fake Check ou translator appelé à la place du handler. Cache Check désactivé (`cache_ttl_seconds=0`).

La frontière test-only reçoit le **message réellement lu sur SQS**. Son corps contient l'enveloppe transport (event_id/event_type/aggregate_id/occurred_at/version/data/metadata), dont `data` contient le JSON enum Telegraph. Désérialiser ce `data` en `TelegraphDomainEvent`, vérifier la concordance IDs/type avec l'enveloppe et l'outbox, puis appeler le vrai trait `EventHandler::handle_event` avec cet événement. Aucune reconstruction par `NotificationCreatedEvent::new` à partir d'une ligne SQL : cela régénérerait une identité et casserait la preuve de corrélation. Le parseur SQS partagé est privé ; ne pas le rendre public pour ce seul test.

Cette jonction prouve transport réel → bytes reçus → vrai handler → vrai droit ; elle ne prouve pas le lifecycle du processus Sentinel ni son consumer daemon. Le test supprime/acquitte le message reçu selon la fixture seulement après succès du handler. Un rejeu réutilise les mêmes bytes et le même event_id.

### 6.1 Exception autorisée — classifier idempotent Sentinel

Autorisation humaine persistée dans `.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/36-autorisation-sentinel-ac6.md` (option du rapport 35) : correctif minimal du classifier, tests positifs/négatifs et revue indépendante bornée. Cette exception ne ratifie pas l'ADR 0313 (`Proposed`) et ne change pas la réalité `Partial` de l'ADR 0301.

- **Surface production exclusive** : classifier et gestion d'erreur idempotente dans `workers/sentinel-sync/src/fga_client.rs`, plus les tests strictement nécessaires, colocalisés ou dans les cibles existantes. Préserver les formats historiques reconnus ; aucune extension générale du worker.
- **Cas réel à couvrir** : après une première écriture réussie, OpenFGA 1.5 renvoie HTTP 400, code `write_failed_due_to_invalid_input`, message `cannot write a tuple which already exists ... invalid write input`. Reconnaître ce motif idempotent précis dans le contexte pertinent de l'opération `write_idempotent`, ainsi que les formats historiques à marqueurs underscored sous leur contexte pertinent. Croiser statut/code et motif avec l'opération concernée ; ni tout HTTP 400, ni une simple mention de doublon/tuple dans un message arbitraire ne vaut succès.
- **Échecs conservés** : autres 400, authn/authz, indisponibilité et réponse ambiguë ou insuffisamment attribuable à l'effet idempotent attendu restent propagés comme erreurs. Ne pas absorber une erreur composite dont le succès de l'effet demandé n'est pas établi.
- **Témoins ciblés** : conserver la preuve rouge AC6 existante du run `t3-routing-2ebb73f602484bdf` ; ajouter avant correction, quand possible, les témoins rouges du classifier. Tests positifs sur le format réel et les formats historiques ; négatifs sur les autres 400, code ou message hors contexte, authn/authz, indisponibilité et ambiguïté. Conserver leurs erreurs observables, pas seulement l'absence de panic. Rapporter noms/commandes réels et rouge/correction/vert sans réécrire la preuve initiale.
- **Oracle AC6 inchangé** : rejouer les mêmes bytes et le même event_id via le vrai `SyncEventHandler`, puis refaire le passage avec ledger mémoire neuf contre le même tuple OpenFGA réel. Aucun skip mémoire substitué ; tuple destinataire seul et aucun nouveau métier. AC1–AC7 restent intégralement requis, sans diminution d'assertions.
- **Gate après diff** : revue indépendante limitée au correctif et aux tests livrés, notamment faux positifs du classifier et conservation du replay réel. Les résultats HTTP/résilience annoncés séparément ne constituent ni une vérification de cette exception ni un PASS global ; le lead les recoupe sans relance documentaire.

Hors scope : API publique, backend, harness ou framework nouveaux/modifiés pour cette exception, refonte du ledger, exactly-once et déduplication IAM. Tout besoin dépassant cette surface revient au lead avant extension.

### 6.2 Calibration déclarée de la fixture INBOUND — preuve ciblée T3

**Jugement documentaire du 2026-10-26 : recevable sous conditions** pour contribuer aux preuves AC2/AC3/AC5/AC6 existantes, sans changer leurs oracles ni dispenser de leurs autres témoins requis. Il s'agit d'une calibration de campagne, pas d'une nouvelle garantie transport ou de production : ces AC vérifient la durabilité/reprise de l'outbox, l'identité de l'événement et le droit réel ; ils n'imposent ni livraison IAM sans redélivrance, ni visibilité INBOUND de 30 s. Le §5.2 garantit l'ownership des tâches locales, pas l'annulation d'un effet réseau distant. Raccourcir la fenêtre de redélivrance dans une fixture réelle est donc distinct d'allonger le délai d'assertion, de renvoyer l'événement ou de masquer une erreur métier ; le seul résultat vert ne suffit pas à cet arbitrage.

- **Calibration explicite et confinée** : sur la seule file IAM INBOUND possédée par cette fixture, lire les attributs, régler `VisibilityTimeout=5` par l'API existante puis relire cette valeur **avant la première app** ; déclarer aussi `DelaySeconds` (observé à 0). Production, défauts du framework, SDK, backend et harness restent inchangés. Aucun changement de visibilité OUTBOUND n'est couvert. L'attente du commit IAM conserve sa deadline absolue de **25 s**, sans remise à zéro. Les sondages d'attributs sont passifs et bornés dans ce même budget ; ils n'ajoutent aucun Receive, renvoi, purge ou attente intercalée entre Stop et Send. Le temps de commit SQL est distingué de la fin des sondages concurrents.
- **Scénario et assertions conservés** : garder l'ordre réellement exercé nominal → stop → indisponibilité de la seule destination OUTBOUND → nouvelle app → second IAM envoyé une seule fois, UUID neuf. Pour la panne : commit métier et vrai `outbox failed` avant destruction du dispatcher/app ; nouvelle instance sur la **même DB**, puis publication du même `NotificationCreated.event_id` et payload. Maintenir les comptes 1/1/1, la corrélation SQL/bytes SQS, le vrai SyncEventHandler, le replay des mêmes bytes/event_id avec ledger neuf, le tuple unique, HTTP 403 sans mutation et HTTP 200 destinataire. Ni skip mémoire ni exactly-once/déduplication IAM ; AC1–AC7 restent inchangés, AC7 local demeure une preuve distincte.
- **Référence du résultat existant, pas nouvelle exécution** : run `t3-exp3-6125e6e3d1d54e21`, artefact `.tmp/t3-exp3-6125e6e3d1d54e21/observations.json` dans le worktree T3 `ensemble-04d9a2f8-zlfadf-t3-runtime` ; source `services/Telegraph/tests/notification_created_routing_test.rs:269–334,393–479,499–651`. L'artefact montre visibilité initiale 30 s / délai 0, puis 5 s relues avant la première app ; invisible=1 aux sondages 0/1/3 s du second IAM, puis 0 à 6/12 s ; même event_id IAM reçu environ 5,75 s après Send et commit SQL à 5,807 s. Les deux parcours nominal/après panne portent un tuple, 403/200 et état lu. Résultat Cargo ciblé recoupé par le lead : 1 PASS, 0 ignoré, 6 filtrés, exit 0 ; ce n'est ni une validation globale ni une exécution par l'architecte.
- **Limites et historique préservés** : les compteurs SQS sont approximatifs ; ces observations corroborent un effet de visibilité/redélivrance, sans identifier le Receive responsable, prouver l'absence d'un bug local ou réparer le SDK. Conserver les deux échecs timeout historiques à 25 s comme preuves distinctes, sans les requalifier en succès. La preuve calibrée ne couvre pas le même scénario à visibilité 30 s, n'établit aucun SLA de livraison 5 s, aucune garantie d'annulation distante et aucune acceptation supplémentaire d'AC7.
- **Gate de rétention** : la pérennisation du réglage test-only exige la décision matérielle du lead et la revue indépendante bornée T4 du diff et des preuves réels. La troisième expérience a précédé l'alternative d'ordre du rapport 46 ; cette alternative non exécutée n'est ni une preuve ni un run supplémentaire requis par cet arbitrage. Aucun quatrième run n'est autorisé ici. Tout besoin de changer une cible/garantie, d'effacer un rouge ou d'étendre production/framework revient à l'arbitrage humain. ADR 0313 reste `Proposed` et ADR 0301 reste `Partial` ; aucune ratification n'est déduite de cette calibration.

## 7. Oracles AC1–AC7 et propriétaires

Tous les comptages sont filtrés par les IDs uniques de la tentative, sur le writer réel ; aucun zéro global d'une DB partagée. Le nom des tests ci-dessous est un **nom à livrer**, pas une preuve d'exécution.

### AC1 — T2, `outbox_tests`

Pour chacun des deux appels : mesurer notification/livraison/outbox avant/après ; succès = deltas 1/0/1 ou 1/1/1 ; payload et aggregate_id concordent avec user_id, notification_id et created_at persistés. Injecter une faute SQL réelle d'insert livraison sur le second et une faute SQL d'insert outbox sur **les deux** (trigger transactionnel de test ou contrainte temporaire dans DB possédée, jamais mock recorder seul). Attendre Err infrastructure/recoverable et deltas 0/0/0. Retirer la faute et prouver un succès. Tester aussi destinataire absent : aucune écriture. L'échec de delivery n'est pas applicable au chemin sans delivery.

### AC2 — T2 + T3

T2 couvre l'état SQL du dispatcher en erreur ; T3 couvre un **vrai client publisher indisponible**, après commit, sans no-op. La création est Ok avec deltas métier uniques, puis la ligne est non-published, possède l'erreur/état retry attendu et reste sélectionnable tant que budget non épuisé. Rétablir seulement la destination sortante (ne pas tuer le SQS partagé qui porte l'entrée IAM), prouver le même événement livré et aucun second métier. Une injection de publisher Err en mémoire n'est qu'un test de composant, pas toute la preuve AC2.

### AC3 — T2, complété T3

Garder le dispatcher arrêté jusqu'au commit ; capturer event_id/payload SQL. Arrêter/détruire sa première instance, créer une instance neuve sur **la même base**, publier, puis vérifier published et message de même identité ; deltas notification/livraison inchangés. Si un claim est laissé publishing, tester la reprise après expiration du verrou avec timings de test bornés ; ne pas supposer qu'un poll immédiat le reprend. Ne pas vider/recréer la table entre instances.

### AC4 — T3, `outbox_runtime_tests`

Cas disabled explicite et cas factory réellement dégradée en NoOp : créer une intention puis observer plusieurs occasions de poll ; status non-published, attempts et payload inchangés, aucun message sortant, HTTP demeure joignable. Prouver la classification non-Live/concrète NoOp : une config enabled=false seule ne couvre pas le cas dégradé. Reconstruire avec vrai transport et prouver la reprise du même event_id. Le garde-fou se teste aussi contre un NoOp qui retourne Ok.

### AC5 — T3, preuve verticale nominale et après panne

Sans aucun grant préalable pour les nouveaux UUID : envoyer `user_email_verified` par la file d'entrée réelle, observer notification + livraison + intention ; recevoir le message sur `test-telegraph-outbound`, vérifier event_id/notification_id/user_id SQL ↔ enveloppe ↔ payload ; le remettre au vrai SyncEventHandler raccordé ci-dessus. Lire les tuples réels : seul `notification:<id>#recipient@user:<destinataire>` est créé. Avant lecture par le destinataire, tenter PUT `<base préfixée>/api/notifications/{id}/read` avec JWT d'un autre utilisateur : HTTP 403 et champs is_read/read_at/updated_at inchangés. Avec JWT destinataire : HTTP 200 et état lu persisté. Refaire le parcours avec un nouvel UUID après panne temporaire réelle du publisher et rétablissement, sans renvoyer l'événement IAM.

### AC6 — T3

Présenter deux fois le même message au handler, même event_id : convergence sans erreur bloquante, tuple destinataire correct et aucun tuple pour l'autre utilisateur ; aucune nouvelle notification/livraison. Refaire le second passage avec un ledger mémoire neuf pour exercer aussi `write_idempotent` contre un tuple déjà présent, pas uniquement le skip du ledger. Cela ne prouve aucune déduplication IAM entrante ou unicité mondiale des traitements.

### AC7 — T3, gate de §5

Exercer la même logique de supervision utilisée en standalone et monolithe sans construire les autres services : tâches actives consumer+dispatcher ; disabled ; sortie Err déjà terminée avant observation ; panic ; arrêt pendant long-poll ; publication en cours. Conserver des témoins de terminaison **des pollers et du parent**, pas seulement `JoinHandle::is_finished()` du parent. Tous les résultats sont consommés, arrêt dans 6 s et aucune activité/poller restant après retour. Le test monolithe doit exercer son vrai helper de supervision/drain, pas une copie test-only. Rapporter les noms réels des nouvelles cibles/tests ; si l'observation des enfants ou leur arrêt n'est pas possible avec les surfaces actuelles, rendre la limite rouge, pas un succès par omission.

## 8. Validation prévue, jamais présentée comme exécutée

T1 : lectures et vérification des liens ; `git diff --check -- docs/contracts docs/adr` par le lead si shell du profil indisponible. Aucun Cargo/runtime T1.

T2 (séquentiel, Windows natif) :

```text
cargo check --locked -p telegraph-service
cargo test --locked -p telegraph-service --test outbox_tests -- --test-threads=1
```

T3, après réutilisation des preuves T2 encore valides :

```text
cargo check --locked --manifest-path runtime/monolith/Cargo.toml
cargo test --locked -p telegraph-service --test outbox_runtime_tests --test notification_created_routing_test --test notification_http_endpoints_test --test event_consumer_resilience_test -- --test-threads=1
cargo fmt -p telegraph-domain -p telegraph-infra -p telegraph-migration -p telegraph-setup -p telegraph-service -- --check
cargo clippy --locked -p telegraph-domain -p telegraph-infra -p telegraph-migration -p telegraph-setup -p telegraph-service --tests -- -D warnings
```

Ajouter au handoff T3 la commande ciblée réelle des tests monolithe/setup créés. Pas de « tout cargo », aucune commande ci-dessus n'a été exécutée par T1. Un seul slot Cargo ; appliquer les 24 jobs convenus par la mission sans override. SQL-only : descripteur PostgreSQL dédié, has_sqs=false, sans SMTP/FGA. Transport/droit : opt-in queues explicite, fixtures PostgreSQL/LocalStack/OpenFGA existantes, MailHog seulement si le harness courant l'exige ; tests serial et aucune IT concurrente manipulant env/singletons.

Avant les IT : lifecycle + evidence skills, inventaire/bail/registre des IDs, contrôle des remplacements transitifs des fixtures ; zéro fixture possédée running à la fin, même sur échec. Aucun nouveau backend ou framework de test. Les erreurs préexistantes sont isolées et rapportées, jamais masquées par réduction d'oracle.

## 9. Sources, contradictions et gates d'intégration

Sources de la base examinée :

- `services/Telegraph/domain/src/service/notification_service.rs:47–116` : deux chemins persist-then-publish ; publication requiert l'ID après persistance.
- `services/Telegraph/domain/src/port/repository.rs:51–107`, `infra/src/repository/notification_write.rs:19–119`, `combined_notification_repository.rs` : ports, writer simple puis transaction notification/livraison sans événement ; mappers aux lignes 89–116 normalisent IDs et dates.
- `services/Telegraph/domain/src/error.rs:173–233` : recoverable et mapping infrastructure existants ; `infra/src/event/processors/notification.rs:29–72` appelle le chemin avec livraison et propage l'erreur.
- `services/Telegraph/setup/src/app.rs:39–128,131–244,278–302` : composition/readiness, pop du consumer, abort non joint, aucun dispatcher ; `crates/readiness/src/factory.rs` et `classify.rs` : statut et transport concrets disponibles sans changer le framework.
- `services/Telegraph/migration/src/lib.rs` : une seule migration ; analogue **IAM** `migration/src/m20220101_000001_initial_schema.rs:20–25` délègue à outbox_migration. Analogue same-txn **Hive** `infra/src/transaction.rs:24–74` ; son mapping Internal n'est pas à recopier.
- `runtime/monolith/src/runtime.rs:65–120,130–204` : collecte des handles, supervision et drain cinq secondes, mais appels stop en amont non bornés globalement.
- `crates/telegraph-events/src/lib.rs` ; `workers/sentinel-sync/src/{lib,handler,fga_client,idempotency}.rs` et `translator/telegraph.rs` : surfaces publiques du handler et tuple recipient existants.
- `services/Telegraph/tests/{common.rs,notification_created_routing_test.rs,notification_http_endpoints_test.rs}`, `config/test.toml` : harness réel, test routing limité à l'existence d'un message, droit nominal préchargé ; destination test spécifique et queues historiquement activées. Le handbook annonce un défaut queue=false qui n'est pas vrai pour cette config (écart 0202 conservé).
- Wiki conception : `obsidian/AI FOR ALL/projects/telegraph/references/telegraph-event-processing.md` décrit la persistance/livraison, pas une garantie outbox. Ni source de ratification ni preuve du code futur.
- Limite de couverture : sous-module rustycog absent du worktree ; les corps `OutboxRecorder::record`, `OutboxDispatcher::{start,claim_batch,publish_claimed,mark_failed}`, `SqsEventConsumer::{start,stop}`, sérialisation SQS et fixture OpenFGA ont été consultés via Serena sur la racine canonique partagée, sans la reconfigurer. **T2/T3 revalident le pin réellement disponible** avant de coder ; seul le sous-lot cancellation-safe du §5.2 et son pin sont autorisés. Le lead a ensuite confirmé le pin exact `6c63b65c23e26b8f83f1c6f63d656ef31c7591d0` et sqs.rs propre, ce qui consolide le constat de lifecycle sans constituer un test.

Gates avant GO T2 : validation/intégration du contrat par le lead ; arbitrage explicite disabled/no-op (recommandation suspendre) ; ratification de la frontière atomique et de la migration ; ratification du budget QA 6 s et des interfaces de lifecycle. La direction cancellation-safe est autorisée par arbitrage utilisateur ; le sous-lot framework/tests/pin précède le code atomicité Telegraph. Sa livraison puis les preuves AC7 T3 restent des gates, pas des faits acquis. Aucune acceptation autonome, aucune promotion globale de 0301 ou de la garantie IAM.

## 10. Photographie locale et preuves bornées — 2026-10-27

### État courant après recherche et nettoyage

Cette section est documentaire : les décisions normatives et AC1–AC7 restent
inchangées. Les sous-sections chronologiques suivantes conservent les résultats
et autorisations **historiques**, pas des consignes actives de relance.

La recommandation retenue conserve les leases SQL et vérifie la reprise après
l'éligibilité persistée. Aucun changement de production contraire n'a été trouvé :
pas de drain ajouté, unlock anticipé, lease raccourcie ou budget d'essais réinitialisé.
Conserver les corrections transactionnelles, NoOp et HTTP ainsi que les deux
témoins composant complémentaires et l'oracle firstFAILED106. Le dispatcher SDK
reste inchangé depuis le pin129 ; le SDK HTTP d60 testé/revu est désormais publié
sur la branche dédiée `mission-telegraph-ac7-zlfadf`, SHA distant vérifié. La
livraison consomme ce commit exact par gitlink, sans changement de branche principale.

Après nettoyage et vérification, le run `post-expiry122-6412a9c40ae64bf4` réussit :
1 PASS/0 échec/0 ignoré/6 filtrés, Cargo0,41,02s libtest. Il atteste15 cycles avant
éligibilité et1 après,1 nouveau claim après expiration,1 succès réel du transport,
1 message SQS corrélé, mêmes identités/payload et comptes métier inchangés.
Les observations terminent30,244094s après l'entrée de reconstruction, avant la
deadline fixe54,989538s ; cleanup inclus dans la phase bornée61s. Le premier
lancement invalide reste historique, pas remplacé par un faux succès rétroactif.
IAM25s et toutes les configurations restent inchangés. Ce résultat composant
seul ne validait pas la verticale finale ni la publication/pin.
Le contrôle fixe25s depuis reconstruction n'est pas un SLA produit couvrant toute
lease30s ; l'arrêt local6s ne libère pas nécessairement un claim durable.

La validation verticale finale134, autorisée séparément, est désormais acquise :
`vertical134-06bd9d7e772c4370`,1 PASS/0 échec/0 ignoré/6 filtrés,25,39s libtest,
Cargo0/lanceur0. Source test80cce027 compilée/revue indépendamment, production
inchangée : reconstruction et réception/validation SQS sous D fixe<=55s selon
éligibilité persistée, IAM25s/arrêt6s/oracles métier conservés. Ce run trouve E
déjà passée : R0/D25s, observations terminées266736µs après t0, puis vrais
replay/ledger neuf/tuple unique/HTTP403sansmutation et200 destinataire atteints.
Nominal et panne sont attestés par deux marqueurs corrélés. Il ne prétend pas
reproduire une lease encore active dans cette verticale ou expliquer109.
Les branches d'erreur/timeout de reconstruction sont revues statiquement, pas
toutes forcées en runtime ; panic arbitraire/annulation extérieure restent hors
garantie générale de jointure. Clippy strict antérieur non vert et Sonar non
exécuté restent des réserves, aucun PASS QA global ou Accept ADR déduit.
Cleanup134 recoupé :4 créés/3 absents/1 SMTP Exited(2) ASK/2 volumes opaques ASK,
0 possédé actif,15 préexistants conservés. Preuves et livraison : rapports139/140.

Contrôle frais de fin :4 fixtures créées,3 absentes,1 SMTP Exited(2) conservée ASK,
2 volumes opaques ASK,0 possédée active/Cargo0 ;14 préexistants conservés.
Le registre corrige en append-only la classification générique yes du SMTP en ASK.
Desktop/WSL conservés pour workload étranger ; RAM non mesurée. Preuves et bilan
borné : rapport132 du dossier de mission, archives privées du run sous TI/.tmp.

La cause exacte109 reste inconnue ;110 et117 ne la reconstituent pas. Les rouges,
sources et résultats archivés restent conservés. Aucun PASS composant ne vaut
acceptation globale, preuve des droits/replay ou livraison du SDK.

### Historique de preuve — non opératoire

Cette photographie actualise les états historiques des §§5.3/9 sans modifier AC1–AC7 ni effacer leurs rouges. T3 outbox est intégrée à `aa400f38608e92a34c463f1f89b02168f8364440` ; l'état HTTP ultérieur est AI `1f2dd30af52a707ceb4abc2fbea3f963307c246f` / SDK local `d60a88b2f2f499768a807941a1b4c80c9a366b3f`. Ce dernier n'est pas publié et le gitlink reste `129dcc1575e7dd59b4c7935de0b3794dbb7e7d07` : ne pas assimiler checkout testé et pin livré. ADR 0313 et 0503 restent Proposed ; ADR 0301 reste Partial, notamment pour l'écart IAM.

### Outbox et transport : preuves historiques conservées

Le writer commun enregistre notification/livraison éventuelle/outbox avant commit ; aucun publish post-commit ne conditionne le résultat métier. Migration additionnelle, sélection Live/non-NoOp et suspension sans faux published sont livrées. La reconstruction sur la même DB reste soumise à la gate verticale finale ci-dessous. Les preuves SQL T2 sont **7 PASS au premier run puis 1 PASS ciblé après correction du nom de table de faute**, pas un run unique 8/8. SDK SQS : 16 tests ; setup : 6 ; monolithe : 2 après correction de fixture IAM. Runtime : 4 dont unité SMTP ; HTTP : 16 dont unité SMTP ; résilience : 2 dont unité SMTP. Ne pas sommer ces répétitions en taux global. Classifier Sentinel : 1 PASS/4 FAILED avant correction, puis 5 PASS et 3 unités privées PASS ; rouge réel de replay conservé.

Le run vertical `t3-exp3-6125e6e3d1d54e21` donne 1 PASS/0 ignoré/6 filtrés/exit 0, nominal et après panne. Il conserve bytes/event_id, nouvelle instance même DB, vrai handler/FGA, replay ledger neuf, tuple unique et 403 sans mutation/200 destinataire. Calibration §6.2 inchangée : INBOUND initialement 30 s, relue à 5 s avant première app, SQL 25 s absolues. Les timeouts de `t3-ac6-063b92d1c2bb4ce2` et `t3-diag-508dac8b6c2442f4` restent inexpliqués, sans correctif SQS revendiqué ; l'alternative d'ordre du rapport 46 n'a pas été exécutée. Ni SLA 5 s ni preuve équivalente à visibilité 30 s.

### HTTP : correctifs locaux et limites de validation

Le défaut E1 réellement rouge sur aa400f3/SDK129 reste historique : parent terminé et shutdown Ok ne prouvaient pas la destruction du handler avant retour. La correction additive du [contrat HTTP](http-runtime-owned-shutdown.md) est désormais adoptée localement en standalone et monolithe : owner distinct, admission/annulation/jointure exhaustive sous budget commun stop inclus ; Drop n'attend pas et ne constitue pas un succès. Les corrections RP-1/RP-2 et C-1 ont leurs preuves/revues ciblées acquises ; elles ne valent pas validation de toute la plateforme.

Les 13 preuves loopback antérieures restent bornées, sans rerun documentaire. La revue indépendante finale du 2026-10-26 à 21:08Z ferme C-2/C-3 sur AI1f2dd30/SDKd60 : C-2 port 0 cleartext, même test rouge puis vert sur le vrai helper ; C-3 CONNECT HTTP/2, comparatif causal avec même suffixe TESTS (baseline composite explicitement non committée), rouge puis vert. Le vert couvre négociation/routage, pas le lifetime de tout upgrade arbitraire. Les tests setup/host sur source finale donnent chacun 1 PASS/0 ignoré ; ce ne sont pas des lancements full app avec DB ou plateforme complète. Aucun protocole global, startup cancellation, TLS invalide, driver panic ou resserrement d'échéances non exercé n'est implicitement validé.

### IT finales et nouveaux blocages — état du 2026-10-27

Le run `http-final-it-4d513ab1f63f499b` (rapport 97), AI1f2dd30/SDKd60, a exécuté les deux scénarios adaptés :

- `outbox_runtime_tests::disabled_and_factory_degraded_noop_preserve_intentions_and_http_then_rebuild_drains` : 1 PASS/0 ignoré/4 filtrés/exit 0 ;
- `notification_created_routing_test::user_email_verified_routes_same_event_through_sentinel_and_real_authorization_after_publisher_recovery` : 0 PASS/1 FAILED/0 ignoré/6 filtrés/exit 101, sur `failed.next_attempt_at > chrono::Utc::now()`. Ce rouge reste conservé ; les valeurs temporelles exactes de cette exécution ne sont pas disponibles.

Le NoOp atteint les cardinalités 0 handle métier suspendu / 2 actifs, owner HTTP distinct et health réel, avec les oracles durables conservés. Aucun vert vertical final n'est déduit du vert historique exp3 ou de cette réussite NoOp.

Diagnostic autorisé unique 105, `vertical-diagnostic-55c9f7ba9837411b`, AI1f2/SDKd60 : 0 PASS/1 FAILED/0 ignoré/6 filtrés/exit 101, même comparaison en échec. Sur la même ligne failed, next_attempt_at−updated_at vaut 4,999997 s ; environ 4,926 s restent à la lecture précoce, mais l'échéance est dépassée d'environ 1,058 s à la comparaison tardive après le sampler passif de 12 s. Cela établit une vérification trop tardive dans **ce run**, pas une cause rétroactivement mesurée pour 97 ni un défaut de production démontré. La suite reprise/replay/droits n'a pas été exécutée dans 105.

La correction temporelle test-only autorisée en106 est intégrée et revue PASS borné : contrôler une date strictement future à l'heure courante dès la première lecture FAILED, dans la future SQL possédée, sous la même deadline absolue25s et sampler inchangé. Ni heure artificielle ni simple `next_at > updated_at` ; ligne absente, erreur SQL ou échéance atteinte font échouer le test. Les contrôles d'identité, erreur, tentatives, persistance, reconstruction, replay, unicité et HTTP403/200 restent conservés, sans snapshot entier figé.

L'unique validation106 est consommée : `vertical-validation106-ac5ddb9998204cdb`, AI1f2 + testSHA `6ac7be0e9e220a501241a0b48bf261c6fd7e14aa921ab098dbbfa29e7df071e1`, SDKd60 propre/index129 inchangé. Résultat : **0 PASS/1 FAILED/0 ignoré/6 filtrés**,50,11s libtest,Cargo101. Le contrôle temporel est franchi, ainsi que les vérifications d'erreur/tentatives/identité avant arrêt. La destination est rétablie et l'application reconstruite, puis `wait_status(PUBLISHED)` expire sous sa limite existante de25s (test556, fixture `outbox_runtime.rs:147`). Aucun changement de production/délais ni relance ; fin reprise/replay/droits du cas panne non atteinte. Le PASS de revue du test ne transforme pas cette exécution en succès.

Le rapport109 distingue une nouvelle frontière de blocage : ancien dispatcher en arrêt, prise en charge SQL et nouvelle instance. La piste d'un verrou publishing de30s face à l'attente de25s reste **hypothèse**, pas cause établie : état et dates exacts de la ligne à cette frontière manquent ; un log de mise à jour annulée ne prouve pas seul l'état persisté. Ni correctif de production ni allongement du délai n'en découlent.

Le diagnostic passif autorisé par110 est consommé : `recovery-diagnostic110-eee8e8b73232492b`, AI1f2/SDKd60, source instrumentée isolée SHA préfixe `5f02f067`, **1 PASS/0 FAILED/0 ignoré/6 filtrés**, 25,76s libtest, Cargo0/launcher0. Production, assertions, délais, backoff, sampler et visibilité inchangés ; aucun autre test exécuté. Le scénario complet atteint publication, replay et vrais contrôles FGA/HTTP403/200 après panne et reconstruction.

Observations sur la même ligne, le même événement et le même agrégat : projection après arrêt failed/attempts1/locked_byNULL/locked_untilNULL, première lecture après reconstruction identique, puis published/attempts2/sans verrou environ109ms après entrée dans l'attente. L'arrêt mesuré dure environ1,491ms dans ce run, pas une borne du pire cas. **La course soupçonnée en109 n'est pas observée** : ce vert prouve ce chemin réussi, ni la cause du timeout109 ni la fiabilité de tous les ordonnancements. L'instrumentation peut modifier le timing ; rouge109 conservé, hypothèse lock30/wait25 ni démontrée ni réfutée globalement.

L'enquête initiale RO, l'auteur et l'analyse du contraste ont confirmé un effort natif high (rapports112/113/116), sans en déduire une preuve comportementale supplémentaire. Le contraste confirme deux chemins différents autour de l'arrêt : candidat trouvé par l'ancien worker juste avant le stop109, contre failed sans verrou aux captures110 puis reprise par le nouveau worker. La cause historique109 reste inconnue ; le cas d'un verrou maintenu après claim est désormais démontré prospectivement par117, sans établir une perte, un défaut d'accès ou une tâche locale orpheline. SDKd60 local non publié/index129 inchangé ; publication et pin final restent bloqués. Le produit conserve le correctif temporel106 (verticalSHA6ac7) et le témoin117, pas l'instrumentation110 isolée.

Le témoin contrôlé `controlled-claim117-695e9e1ccbdd409c` est exécuté une seule fois : **1 PASS/0 FAILED/0 ignoré/5 filtrés**,35,65s libtest,Cargo0/launcher0. SourceSHA403e8c90/patch99ecdf11, revue indépendante bornée favorable, aucun changement production/délais. ClaimSQL terminé, annulation locale possédée, publisher et parent détruits/joints ; même ligne publishing/attempts1/ancienworker/verrou30s. Le nouveau pilote composant fait13sélections réelles commencées et terminées,0claim,13états corrélés inchangés ; aucun message observé lors de la sonde SQS ponctuelle. Fenêtre25001ms, terminée avec environ4,984s de verrou restant. Ce PASS atteste le pronostic du composant, **pas une publication sous25s**, ni la cause historique109, l'application entière, le SQL interrompu ou l'envoi après expiration. Le rouge109 n'est pas remplacé ; aucune exécution, correction ou publication supplémentaire autorisée. Une consultation du canon détermine la prochaine proposition bornée avant tout nouvel essai.

Avis architectural121, lecture seule/high : §3 et AC3 prévoient déjà la reprise après expiration du claim ; AC7 exige la jointure locale sous6s, pas la libérationSQL immédiate. Le contrôle PUBLISHED25s depuis reconstruction ne couvre pas tous les états légalement retardés par lease30s et n'est pas un SLA produit ; le rouge exécuté109 reste inchangé. L'utilisateur autorise explicitement en122 une préparation tests-only puis une seule preuve composant post-expiration : t0 à l'entrée de reconstruction incluse, résiduel R mesuré une fois sur locked_until/next_at avec0<=R<=30s, deadline fixe t0+R+25s<=t0+55s, sans reset et cleanup6s inchangé. IAM25s, production/configuration et critères d'identité/transport restent inchangés. La source compilée et revue445f729a corrige T-1 par contrôle strict après le dernier await. Sa tentative `component-post-expiry122-72aba6ce2cf643b2` s'arrête en initialisation : runID du lanceur41caractères rejeté par le maximum40 du contextefixture, aucun conteneur ou marqueur de preuve, libtest0PASS/1FAIL/0ignored/6filtered/0,00s. **La preuve après expiration reste absente**, ce n'est pas un échec fonctionnel produit. Le préfixe est corrigé à31caractères et validé avantCargo, sans changerSDK/test/budgets ; une exécution de remplacement nécessite encore l'accord humain127. Aucune nouvelle verticale ou publication couverte ; toute reprise immédiate à l'arrêt demanderait un mandat produit/concurrence distinct.

### Provenance, ressources et réserves

Preuves brutes privées : archives `.tmp/telegraph-outbox-{t2,t3}-evidence/manifest.json`, summaries et notes de validation ; rapports97/105/109/110/115 et briefings HTTP/tests conservés par le lead. Preuves110 sous TI/.tmp/recovery-diagnostic110-results-eee8e8b73232492b/, dix copies aux empreintes vérifiées puis cleanup indépendant ; source/patch/compilation/plan archivés séparément. Ce ne sont pas des liens publics ni une invitation à publier les logs/ledgers. Clippy strict Sentinel et fmt ciblé ont des logs verts ; Clippy Telegraph (deux dead_code configuration) et SDK (diagnostics préexistants) restent non verts, sans masque. Sonar non exécuté ; aucun quitus QA global ni Accept ADR déduit.

Cleanup97 vérifié : 8 créés/6 retirés, 2 SMTP Exited(2) ASK, 4 volumes opaques ASK, 0 possédé actif, 8 préexistants inchangés. Cleanup105 : 4 créés/3 absents, 1 SMTP Exited(2) ASK, 2 volumes ASK, 0 possédé actif/Cargo0, 10 préexistants inchangés. Cleanup109 indépendamment vérifié : 4 créés/3 absents, 1 SMTP Exited(2) ASK, 2 volumes ASK, 0 possédé actif/Cargo0, 11 préexistants mêmes identités/états. Cleanup110 indépendamment vérifié : 4 créés/3 absents, 1 SMTP Exited(2) ASK, 2 volumes opaques ASK, 0 possédé actif/Cargo0, 12 préexistants mêmes identités/états. Node étranger préservé ; aucun arrêt global Docker/WSL ni suppression de volume. Comptes séparés, sans adoption des préexistants ni taux global de réussite.

Cleanup117 indépendamment vérifié : 4 créés/3 absents, 1 SMTP Exited(2) ASK, 2 volumes opaques ASK, 0 possédé actif/Cargo0, 13 préexistants mêmes identités/états. Preuves privéesTI/.tmp/controlled-claim117-695e9e1ccbdd409c/, dont lead-cleanup-verification.json ; source/patch/compilation et revue archivés séparément. Aucun bilan de campagne n'autorise à adopter les ressources anciennes ou à présumer un inventaire futur.

Le rollup historique T3 (47créés/40retirés/7SMTPExited(2)ASK/16volumes opaquesUNKNOWNask/0possédé running) n'est pas l'inventaire des campagnes97/105/109 ou du diagnostic110. Les comptes restent séparés ; preuves109 sous TI/.tmp/vertical-validation106-ac5ddb9998204cdb/ (archives privées). Desktop global retenu pour le workload étranger, RAM non mesurée ; aucune suppression/prune nouvelle revendiquée. Limites inchangées : essais finis, pas exactly-once/déduplication IAM/backfill/fencing multi-worker, pas annulation d'effets distants, pas arrêt synchrone non coopératif/spawn_blocking/kill OS garanti.
