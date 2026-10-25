# Contrat — création et publication durable des notifications Telegraph

- État : **Proposed**, soumis au lead et à ratification humaine ; ne vaut ni Accept ni GO T2.
- Date : 2026-10-25 ; jalon : fiabilité Telegraph (hors P0–P6 Apparatus).
- Base examinée : `93e376c5530a439aaafe61a64d3e881865c1acf1` (Git du worktree vérifié par le lead).
- Canon : [0301](../adr/0301-outbox-transactionnel-rustycog.md), [0300](../adr/0300-crates-events-contrat-sans-transport.md), [0102](../adr/0102-setup-composition-root.md), [0200](../adr/0200-it-infra-reelle-rustycog-testing.md), [0201](../adr/0201-mocks-http-sortant-seulement.md), [0202](../adr/0202-transport-opt-in-queues-desactivees.md), [0303](../adr/0303-sentinel-sync-worker-fga.md).
- Décision complémentaire recommandée : **ADR-0313**, à matérialiser par le lead avec son digest Serena au même tour. Aucun fichier ADR-0313 n'est créé par ce contrat. 0301 reste sa photographie Accepted / Partial, inchangée.

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

Déclencher l'arrêt du consumer et du dispatcher, puis attendre **tous** les handles possédés. En monolithe, inclure ces handles dans la supervision et le drain existants ; ne pas appeler `TelegraphApp::run()`. L'erreur d'origine et les erreurs de cleanup restent observables. Le cleanup HTTP possède également son handle (abort éventuel puis join), sans tâche orpheline après le retour ordonné.

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

## 6. Raccordement test-only à Sentinel (T3)

Ne démarrer ni le binaire Sentinel ni toute la plateforme. Ajouter si nécessaire **uniquement une dev-dependency** de `telegraph-service` vers `workers/sentinel-sync` (crate `sentinel-sync`) et `telegraph-events`. Sa lib expose déjà `handler`, `translator`, `idempotency`, `fga_client`. Aucun changement production Sentinel ; ce raccordement ne nécessite aucun changement framework supplémentaire au sous-lot §5.2.

Construire dans la fixture de test le vrai `SyncEventHandler::new` avec : `TelegraphTranslator::new()`, `InMemoryEventLedger::new()` derrière `Arc<dyn EventLedger>`, et `OpenFgaWriteClient::new(openfga.client_config())`. La fixture expose le client config du **même** store/modèle OpenFGA réel que le checker HTTP. Pas de tuple préchargé, `allow`, `allow_all`, fake Check ou translator appelé à la place du handler. Cache Check désactivé (`cache_ttl_seconds=0`).

La frontière test-only reçoit le **message réellement lu sur SQS**. Son corps contient l'enveloppe transport (event_id/event_type/aggregate_id/occurred_at/version/data/metadata), dont `data` contient le JSON enum Telegraph. Désérialiser ce `data` en `TelegraphDomainEvent`, vérifier la concordance IDs/type avec l'enveloppe et l'outbox, puis appeler le vrai trait `EventHandler::handle_event` avec cet événement. Aucune reconstruction par `NotificationCreatedEvent::new` à partir d'une ligne SQL : cela régénérerait une identité et casserait la preuve de corrélation. Le parseur SQS partagé est privé ; ne pas le rendre public pour ce seul test.

Cette jonction prouve transport réel → bytes reçus → vrai handler → vrai droit ; elle ne prouve pas le lifecycle du processus Sentinel ni son consumer daemon. Le test supprime/acquitte le message reçu selon la fixture seulement après succès du handler. Un rejeu réutilise les mêmes bytes et le même event_id.

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
