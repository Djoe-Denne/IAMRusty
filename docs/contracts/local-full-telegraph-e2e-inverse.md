# Local-full — inverse Telegraph avant signup : impossibilité bornée

- Date : 2026-11-03 ; jalon : départ mesh E2E sur le local-full existant,
  hors P0–P6 Apparatus.
- Base source vérifiée : `7fdb17d33601ce85995fac8f1d89bef68d3c6b97` ;
  worktree documentaire au même HEAD, sans modification applicative.
- État : **constat sourcé ; inverse non qualifié**. Les exigences d'une solution
  future ci-dessous sont **Proposed**, pas une API, une migration ou un lot Rust
  autorisé. Trois options sont comparées au §4 ; aucune n'est ratifiée/sélectionnée.
- Complète le [raccord E2E existant](local-full-existing-e2e-wiring.md), sans en
  lever les gardes ni ratifier ADR0606. Parent seul sur index, liens entrants et
  ledger ; son intégration doit référencer ce contrat depuis l'index existant.

## 1. Décision pour le périmètre actuellement autorisé

**Conserver `Database.qualify_notifications_cleanup` bloquant avant signup.**
Les surfaces actuelles ne permettent pas de prouver conjointement la provenance
exhaustive des créations asynchrones et l'absence de création/reprojection après
nettoyage. Aucune substitution purement Ops compatible n'est qualifiée ici.

Ce constat satisfait le lot de conception A2, **pas** la demande de départ E2E.
L'intégration du contrat GitHub additif ne suffit donc pas à autoriser signup.
Ni suppression de garde, purge filtrée par user, délai de silence, désactivation
de queue, arrêt/rollout des services partagés, ni réduction des 39 cas n'est une
solution admise. Aucun signup, SQL destructif ou changement runtime n'a été fait.

La reprise vers une solution exige un arbitrage explicite sur la frontière
manquante (§4), puis un contrat exécutable intégré et ses preuves. Ne pas publier
un writer Telegraph à partir de ce seul constat.
Une autre orientation, **C**, demande à l'utilisateur une exception locale de
rétention plutôt qu'un inverse : elle n'est pas autorisée par le mandat actuel.

## 2. Sources confrontées : ce qui est acquis, et ce qui manque

### Protection Ops réelle

- [`local_full_e2e.py`](../../ops/tests/local_full_e2e.py),
  `Database.qualify_notifications_cleanup` (585–593), lève toujours
  `Blocked(NOTIFICATION_CLEANUP_GAP)` sans exécuter PostgreSQL.
- `Mesh.signup` (658–686) vérifie le run ouvert et le registry IAM, puis qualifie
  cet inverse **avant** qualification SQL, recherche MailHog, `signup-intent` et
  `POST /iam/api/auth/signup`. Le registry actuellement contrôlé par ce code exige
  GitHub ; la conservation effective de GitLab reste une obligation du raccord
  additif, pas un fait déduit de ce contrôle.
- `run_action` (1029–1125) accumule les préconditions pendant `prepare-probes` ;
  leur présence garde `prepared=false` et `executed=0`. Un reçu `prepared` présent
  n'est pas, à lui seul, une qualification réussie.
- `cleanup` (899–1006) refuse les comptes historiques/non qualifiés et inscrit
  `telegraph-notifications-unqualified`, sans prétendre un inverse exécuté.
  Conserver reçu/résidus ; ne pas adopter leurs lignes par email ou user_id.

Les trois régressions de
[`test_local_full_e2e.py`](../../ops/tests/test_local_full_e2e.py) à conserver sont
`test_real_telegraph_inverse_gap_blocks_before_signup_without_pg_exec`,
`test_unqualified_historical_notifications_never_report_cleanup_pass` et
`test_prepare_current_gitlab_and_notification_gap_never_prepared_true`.
Elles ont été **lues**, non exécutées pour ce lot documentaire.

### Corrélation entrante disponible seulement pendant le traitement

- [`ProcessEventCommand`](../../services/Telegraph/application/src/command/process_event.rs)
  conserve `original_event_id` (48–59, 109–113), distinct de son nouvel ID de
  commande. Le mapper
  [`EventContext`](../../services/Telegraph/application/src/usecase/event_processing.rs)
  (77–89) le transmet bien : l'identité entrante n'est pas absente partout.
- [`DatabaseNotificationProcessor::save_notification_communication`](../../services/Telegraph/infra/src/event/processors/notification.rs)
  (28–73) reçoit cet ID, le journalise, puis retourne `Result<(), DomainError>`.
  Ni retour durable des PK au runner ni acquittement de fermeture par run.
- [`CommunicationFactory::build_notification_communication`](../../services/Telegraph/domain/src/service/communication_factory.rs)
  (108–179) construit `id=None`, sans transporter l'ID entrant dans la création.
- [`NotificationWriteRepositoryImpl::create_atomically`](../../services/Telegraph/infra/src/repository/notification_write.rs)
  (34–151) alloue la PK notification, la delivery éventuelle, puis un nouvel
  événement sortant ; son entrée ne contient ni run ni source-event ID.
- Le [schéma notifications/deliveries](../../services/Telegraph/migration/src/m20250201_000001_create_notification_tables.rs)
  porte `user_id` et les PK/FK, pas de relation persistée source-event → création.
  La FK delivery a `ON DELETE CASCADE` : supprimer une notification peut aussi
  retirer une delivery non recensée ; la seule PK parent n'autorise pas cela.

Un log associant les deux IDs est un diagnostic, pas une autorité transactionnelle
exhaustive récupérable après perte d'ACK, crash ou redelivery. L'UUID utilisateur,
le contenu, une fenêtre temporelle ou un snapshot de lignes ne prouvent ni le
créateur du run ni toutes les créations encore à venir.

### Outbox sortante : garantie différente, conservée

Le [contrat outbox](telegraph-notification-outbox.md) et
[ADR0313](../adr/0313-telegraph-notification-outbox-atomique.md) restent
**Proposed / Implemented**, pas Accepted. Ils garantissent la transaction writer
notification + delivery éventuelle + intention `NotificationCreated` et la reprise
du **même événement sortant**. [ADR0301](../adr/0301-outbox-transactionnel-rustycog.md)
reste Accepted / Partial, notamment pour IAM.

[`NotificationCreatedEvent::new`](../../crates/telegraph-events/src/lib.rs)
(22–38) reçoit notification/user/date, pas l'event_id IAM ni le run. L'atomicité
sortante ne prouve donc ni déduplication entrante ni fin des consommateurs.
`published` n'est pas un ACK de projection Sentinel. Le tuple
`notification:<id>#recipient@user:<user>` est lui aussi un effet à inventorier :
[`TelegraphTranslator`](../../workers/sentinel-sync/src/translator/telegraph.rs).
Supprimer seulement les lignes SQL ne retire pas ce tuple, et un événement
sortant retardé peut le réécrire après une suppression.

[`TelegraphEventHandler::handle_event`](../../services/Telegraph/infra/src/event/consumer.rs)
(153–195) termine la commande et rend Ok/Err ; aucun reçu par run n'est exposé.
L'arrêt possédé local de
[`TelegraphApp`](../../services/Telegraph/setup/src/app.rs) et
[ADR0503](../adr/0503-http-runtime-owned-shutdown.md) (**Proposed / Implemented**)
ne clôt ni les producteurs IAM distants ni les messages déjà envoyés/invisibles.
Le budget d'arrêt local n'est pas une période après laquelle un cleanup est sûr.

### Autorité et analogues

- [ADR0403](../adr/0403-telegraph-notifications-event-driven.md),
  **Accepted / Implemented** : HTTP étroit lecture/mark-read. Le
  [router réel](../../services/Telegraph/http/src/lib.rs) (30–49) ne monte ni
  suppression ni clôture par run ; mark-read n'est pas un inverse.
- [ADR0200](../adr/0200-it-infra-reelle-rustycog-testing.md) et
  [0201](../adr/0201-mocks-http-sortant-seulement.md), Accepted / Implemented :
  serveur/DB/OpenFGA réels pour les futures IT aux frontières concernées.
  [0202](../adr/0202-transport-opt-in-queues-desactivees.md), Accepted / Partial :
  transport opt-in, exception historique Telegraph conservée ; aucun silence de
  queue désactivée ne remplace une preuve réelle de fin des consommateurs.
- Autre service : le [ledger Sentinel](../../workers/sentinel-sync/src/idempotency.rs)
  offre des implémentations mémoire/Postgres et le
  [handler](../../workers/sentinel-sync/src/handler.rs) suit sa projection.
  Ces surfaces sont distinctes du writer Telegraph : les réutiliser ne rendrait
  pas spontanément atomiques provenance, création SQL et effets OpenFGA.
- Le [handbook événements](../platform/events-outbox.md) distingue bien entrée
  IAM, sortie Telegraph et projection Sentinel ; ses preuves de replay sont
  sortantes. QMD, collection `aiforall-wiki` (283 documents, état 2026-10-29), a
  permis de lire la [note de conception](../../obsidian/AI%20FOR%20ALL/projects/telegraph/references/telegraph-event-processing.md),
  §§Service-Specific Differences/Open Questions : pipeline et erreurs partielles,
  aucune garantie de fermeture du run. Cette note n'est pas une ratification.

**Écarts explicites :** les commentaires Ops citent des numéros historiques du
writer, désormais déplacés par l'outbox ; le manque de corrélation demeure après
lecture des corps actuels. Le mot « corrélé » des preuves outbox désigne SQL ↔
bytes sortants, pas run ↔ événement IAM ↔ toutes les créations. Aucun écart n'est
« réparé » dans une ADR Accepted ou une source applicative par ce lot.

## 3. Contre-exemples qui empêchent un inverse compatible

1. **Tardif/rejoué :** après lecture puis DELETE user-filtered, un message IAM
   retardé/invisible ou rejoué peut créer une nouvelle PK. Une liste momentanément
   vide et des compteurs de queue ne ferment pas ce futur.
2. **ACK perdu :** commit Telegraph réussi puis perte d'ACK/arrêt avant retour :
   le runner ne connaît pas les PK, et le même événement peut être retraité.
   Une recherche par user/event log n'est pas un ACK de propriété.
3. **Concurrence :** le mutex du runner ne clôt pas les consumers/applications ;
   un insert ou une projection peut s'intercaler entre vérification et retrait.
   Verrouiller les seules lignes existantes ne bloque pas une création future.
4. **Étranger :** une ligne ajoutée au même user par un autre acteur n'est pas
   possédée par le run. Filtrer sur user, email, titre/date ou cascade serait
   une adoption ; un snapshot différent doit bloquer, pas élargir la purge.
5. **Échec partiel/reprise :** notifications SQL supprimées mais outbox encore
   envoyable, mail tardif ou tuple Sentinel encore présent : un zéro SQL final
   ne prouve pas le cleanup. Un restart sans registre causal durable perd
   précisément les informations nécessaires pour décider sans adoption.

## 4. Options de solution future — Proposed, à arbitrer avant implémentation

Pour fournir un inverse (A/B), le futur arbitrage doit couvrir **ensemble** les
points suivants. C propose explicitement de ne pas fournir cet inverse, avec une
exception utilisateur préalable et les limites décrites plus bas.

- Provenance durable et authentifiée : run/cible/epoch, ensemble fermé des
  événements entrants (ID + type + empreinte), leurs créations notification,
  delivery, intention/event sortant et effets aval. Le client ne doit pas pouvoir
  étiqueter une ligne étrangère comme sienne ; mêmes IDs avec contenu contradictoire
  = refus, pas adoption.
- Acquisition de propriété avant mutation et enregistrement atomique avec ses
  effets locaux ; perte d'ACK doit rester récupérable sur cette autorité durable,
  ou bloquée explicitement. Ni génération d'un nouvel ID ni retry aveugle.
- Fermeture causale empêchant de nouvelles écritures du run, y compris après
  replay/restart : admission des producteurs, messages en transit/invisibles,
  handlers concurrents et callbacks/dispatchers. Un tombstone ou une barrière
  serait un **nouveau contrat à décider**, pas une fonctionnalité disponible.
- Traitement de l'aval : publication acceptée avant perte d'ACK, reprise de lease,
  projection Sentinel tardive et effets email déjà émis. Ne pas purger une queue,
  outbox/audit globaux ou mémoire de déduplication pour obtenir zéro.
- Inverse par identités attestées avec préconditions atomiques côté writer,
  contrôle exhaustif des dependants avant cascade, refus de changement/conflit
  et reprise à chaque étape sans re-créer ni réadopter des lignes.
- Reçu privé append-only conservant progression/incertitude/résidus ; clôture
  seulement après preuves de fermeture et d'inverses. Absence observée seule
  n'autorise pas à marquer une suppression dont l'ACK a été perdu comme acquise.
  Exclure tokens, corps email, mots de passe, OAuth/HMAC et clés des artefacts.

### Option A — manifeste d'événements et registre d'effets hors payload (recommandée)

**Recommandation de conception seulement :** ajouter une autorité durable de run
et des registres locaux d'effets, en conservant les formats d'événements IAM et
Telegraph. C'est le minimum recommandé **en changement de contrats d'événements**,
pas une petite correction Ops ni une solution disponible aujourd'hui.

1. Avant signup, le parent trusted réserve un run neuf lié au bail/cible/epoch.
   Une interface privée spécialisée, authentifiée et non exposée au client métier
   lie cette réservation à la commande de création effectivement admise par IAM.
   La liaison run/commande/compte et les IDs des événements émis doivent être
   enregistrés au writer avec leur provenance ; jamais reconstruits par recherche
   d'email ou par lecture globale d'outbox. Un conflit d'absence/nonce refuse la
   création, sans adopter un compte préexistant. Les commandes ultérieures de ce
   run rejoignent la même autorité, sans devenir des flags publics libres.
2. Un registre Telegraph indexé par `(source, event_id, processeur)` contient
   l'empreinte d'entrée et les PK notification/delivery/outbox + event_id sortant.
   Son passage à `committed` et les effets SQL sont dans **la même transaction**.
   Le processeur email participe aussi au registre d'effets, avec état d'envoi
   distinct. Dupliquer le même événement ne crée pas un nouveau métier ; contenu
   contradictoire ou tentative concurrente ne s'approprie pas le premier résultat.
3. La clôture ferme l'admission des **commandes du run** côté IAM et scelle la
   liste exhaustive d'IDs émis. Une barrière persistée Telegraph ferme les IDs
   correspondants ; acquisition de traitement et clôture utilisent la même
   exclusion transactionnelle. Un handler déjà admis doit terminer ou rester
   explicitement incertain avant ACK de fermeture ; un événement tardif connu
   est rejeté/reconnu terminal sans réécriture. Aucun arrêt global de consumer.
4. Le manifeste transmet séparément les IDs sortants à Sentinel, dont un registre
   durable clôt la projection et l'inverse du tuple exact sous la même exclusion
   locale. Dispatcher encore en lease/envoi et replay après restart ne doivent
   plus pouvoir recréer un effet : état terminal durable côté sink, pas purge des
   messages ni supposition `published = projeté`. Les tombstones survivent au run.
5. L'inverse lit ce manifeste durable et supprime seulement les PK/effets attestés,
   avec préconditions atomiques et journal de retrait. Une ligne du même user
   absente du manifeste est étrangère : préservation, résidu et blocage de la
   suppression de compte si elle en dépend. Le protocole ne s'arroge donc pas
   toutes les activités d'un user. Source/commande non recensée ou fermeture
   partielle interdit un cleanup PASS.

**Stockage et interfaces à autoriser :** nouvelle réservation/manifeste sur le
writer IAM ; nouveau registre effets/fermeture dans la DB Telegraph ; état de
clôture/inverse dans le ledger durable Sentinel. Pas de nouvelle DB, backend ou
service. Interface privée fermée `reserve → bind → seal → close → inverse →
status`, vérification de génération/CAS, acteurs autorisés et port d'exposition
à ratifier ; ces noms sont conceptuels, pas des endpoints livrés ni une permission
SQL. Un compte créé dont l'ACK runner est perdu reste bloqué sauf preuve provenant
de cette autorité transactionnelle ; aucun lookup email ne remplace cette preuve.

**Impacts concrets :** Ops `local_full_e2e.py` et reçu ; IAM
`domain/src/service/{auth_service,registration_service}.rs`,
`infra/src/transaction.rs`, ports et migration à définir ; Telegraph
`application/src/{command/process_event,usecase/event_processing}.rs`,
`domain/src/port/{event_handler,repository}.rs`,
`infra/src/event/processors/{email,notification}.rs`, writer notification,
consumer/setup et nouvelle migration ; Sentinel `handler.rs`, `idempotency.rs`,
`translator/telegraph.rs`, accès FGA et migration de son ledger selon sa surface.
Renforcer le port writer conserve l'atomicité sortante de0313, sans prétendre que
ce dernier inclut déjà le registre entrant. Le port IAM actuel
[`IamOutboxUnitOfWorkImpl::record_event`](../../services/IAMRusty/infra/src/transaction.rs)
ouvre sa propre transaction (21–48) : une liaison atomique à la création de compte
n'est **pas** acquise. Ce sous-problème est un coût explicite à arbitrer, pas un
prétexte pour recopier le défaut documenté par0301.

**Limites à contractualiser avant code :** pas de transaction distribuée ni
exactly-once SMTP ; un SMTP accepté avec réponse perdue reste `uncertain` tant
qu'une preuve finie n'établit pas les messages exacts. MailHog n'acquiert pas un
nouveau token causal par magie. L'arbitrage doit soit fournir cette preuve via
une couture compatible, soit conserver le résidu bloquant. Aucune rétention de
tombstone par délai arbitraire : sa durée/collecte doit couvrir les replays
autorisés ou conserver un refus durable. Toute migration conserve les lignes
historiques non attribuées ; aucun backfill qui invente une propriété.

### Option B — contexte de run propagé dans les événements de bout en bout

Ajouter un contexte causal explicite et versionné `(run, cible, epoch,
commande/source-event)` aux événements IAM puis Telegraph, et le propager à tous
les effets/consommateurs concernés. Le contexte provient exclusivement de
l'autorité trusted de réservation, jamais d'un simple header fourni par le client.
Le message de clôture porte un manifeste scellé des événements attendus : un
simple marqueur « dernier message » n'est pas une barrière dans une file partagée.
Les consumers tiennent leur ledger de traitement/fermeture, dédupliquent sous
transaction locale et accusent le manifeste après terminaison de ses effets.
Les ACK manquants/partiels restent visibles et récupérables par statut durable.

**Différence avec A :** attribution embarquée dans le contrat transport, au lieu
d'une jonction privée par ID existant. Elle simplifie le diagnostic causal et un
éventuel futur usage multi-producteurs, mais ne dispense **ni** de l'admission
producer fermée, **ni** des registres/tombstones/ACK et de la protection des
événements étrangers. Ordre d'arrivée, ACK broker et zéro queue ne suffisent pas.
L'incertitude SMTP de A reste une limite à traiter, pas un exactly-once implicite.

**Impacts supplémentaires :** `crates/iam-events/src/lib.rs`,
`crates/telegraph-events/src/lib.rs` et leurs sérialiseurs/versions, producteurs
IAM, consumers Telegraph/Sentinel et tests de compatibilité/replay ; schéma de
contexte persisté dans les mêmes trois stockages locaux et nouveau contrat de
message de clôture/ACK. Les signatures/intégrité transport doivent couvrir le
contexte, sans nouveau secret de run en logs. Anciens événements sans contexte
restent non attribués ; ils ne peuvent ni fermer un run ni être purgés avec lui.
Le modèle FGA n'est pas tacitement modifié ; seul un inverse exact explicitement
autorisé peut retirer le tuple recipient attesté.

**Option non recommandée pour ce mandat :** elle fait du run E2E une évolution
des contrats interservices et impose compatibilité/versions à des consommateurs
non nécessaires au simple départ mesh. Ce coût ne se justifie que par une demande
explicite de corrélation produit réutilisable ; pas par l'urgence de démarrer.

### Option C — rétention locale assumée, sans inverse (à arbitrer)

**Cohérente pour le seul lancement local sous exception utilisateur explicite**,
pas aujourd'hui : un mode opt-in versionné `RETAINED` conserve les données/effets
du run, déclare le cleanup métier **non PASS** et ne prétend ni les inventorier
exhaustivement ni fermer les consommateurs. Il change l'obligation du
[contrat existant §2](local-full-existing-e2e-wiring.md#2-trois-actions-existantes-entrées-privées-aucun-setup-caché)
(inverse qualifié avant création), pas une garantie produit. Aucun invariant
Accepted identifié n'impose une purge des fixtures d'un run local ; en revanche
ce contrat et les gardes actuelles interdisent de lancer ainsi **sans arbitrage**.

- Autoriser seulement un run UUID neuf sur la cible locale existante attestée
  (bail/UID/epoch/HEAD/inputs liés), sans nouveau cluster ni adoption de compte,
  ancienne fixture ou run historique. Mode et accord de rétention liés au reçu
  privé **avant** toute mutation ; défaut strict inchangé, aucun fallback après
  échec de qualification. Le schéma fermé `read_inputs` v1 (167–180) ne connaît
  pas ce mode : variante explicite à décider, pas une clé ignorée/flag caché.
- Un intent d'exécution consommable une seule fois est persisté avant mutation,
  puis un état terminal privé `RETAINED`, y compris en échec/incertitude. Tous
  les points d'entrée/mutation doivent refuser ce run après consommation ou
  rétention, aussi après crash ; `execution-started` empêche déjà le second
  `run_action`, mais `assert_run_open` (162–164) ne connaît actuellement que
  `cleanup.complete=true`. Ne pas fabriquer ce dernier pour fermer le run :
  nouveau terminal distinct, refus de nouveau reçu utilisant le même run_id et
  reprise limitée au rapport/à la fermeture des seuls processus possédés.
- Le `finally` reste exécuté mais ne fait **aucun inverse métier** : ni DELETE
  organisation, suppression provider SQL, DELETE MailHog, ni inverse compte.
  Tous quatre sont sous `cleanup` (899–1006), qui retourne actuellement **avant**
  eux lorsque Telegraph est non qualifié (913–918). Retirer cette seule garde
  rendrait ces suppressions accessibles et ne produit donc pas C. Ne pas non
  plus déclencher le login de récupération de l'inverse organisation, qui créerait
  de nouvelles sessions. Qualifier séparément les seules opérations de lecture/
  fixture effectivement nécessaires ; aucune qualification d'inverse fictive.
- Le driver [`local-full-acceptance.py`](../../ops/tests/local-full-acceptance.py)
  délègue `prepare-probes/e2e/cleanup-probes` à `run_action` (175–188), et exige
  aujourd'hui cleanup PASS pour un bilan global PASS (520–525). Sa variante
  proposée doit rendre `RETAINED`, résidus et code non nul, sans déplacer le
  dénominateur. Le cleanup legacy vide (178–180) n'est jamais un reçu de rétention
  ou un inverse. `prepare-probes` actuel ne crée aucune ressource et déclare
  `created_resources=[]` ; C n'autorise aucune probe K8s additionnelle.
- Conserver le cleanup **technique** : les contextmanagers
  [`e2e_forward`](../../ops/deploy/local_full_target.py) (1453–1491) ferment/jointent
  leurs enfants et vérifient incarnation/mapping ; le
  [`operation_lock`](../../ops/deploy/local-full-storage.py) (27–37) retire son
  mutex, pas les données. Ne pas supprimer ces `finally`, laisser des tunnels,
  stopper les services partagés ou confondre rétention métier et keepalive runtime.

**Limite impérative sur « aucun DELETE métier » :** interprété littéralement sur
tout le parcours, C est **incompatible avec mesh39** : `Mesh.s2s` (833–844) doit
exécuter le **DELETE revoke IAM** autorisé et vérifier le retrait du provider,
puis token404. Ce sont des oracles requis, pas du cleanup. Seule l'interprétation
« aucun DELETE de nettoyage, oracles métier inchangés » est proposée ; si
l'utilisateur exige zéro DELETE y compris revoke, rejeter C, sans retirer ces cas.

**Risques assumés à faire accepter :** comptes/sessions/refresh non révoqués hors
oracles, organisations/membership, mails et liens de vérification, notifications,
deliveries, outbox/tuples FGA et créations tardives restent possibles. Garder le
reçu owner-only, identités ACKées et incertitudes sans tokens/corps/clefs/HMAC ;
interdire export de credentials, réemploi et nouvelle campagne implicite. Aucun
délai ni expiration de token n'est une preuve de cleanup ; coût/disque et
responsabilité d'une disposition ultérieure explicite des résidus à attribuer.
Échec d'écriture du reçu = incertitude bloquante, pas autorisation de rerun.

**Portée/recommandation :** pour le seul départ demandé, **C est l'option la plus
proportionnée à soumettre d'abord à l'utilisateur**, si ce risque local lui est
acceptable. Delta strictement test-side proposé : `local_full_e2e.py`, driver,
reçus privés et tests Ops (mode fermé, terminal, crash/ACK, refus de rejouer,
zéro inverse métier et fermeture effective des tunnels) ; pas trois migrations,
nouveau contrat d'événement, SDK ou FGA. A demeure la recommandation si un
**véritable inverse** est exigé ; B ne se justifie que pour un besoin causal
interservices plus large. Cette comparaison n'autorise aucun de ces deltas.

### Alternative rejetée et décision humaine bornée

**Rejetée :** UUID notification déterministe à partir de l'event_id IAM puis
DELETE par IDs déduits, sans registre de fermeture. Cela ne révèle ni deliveries
réellement créées, ni effets SMTP/FGA, ni provenance des événements, et un replay
peut recréer les mêmes IDs après DELETE. Même défaut avec purge user-filtered ou
attente de silence ; ni l'unicité d'une PK ni l'outbox sortante ne ferme l'entrée.

**Arbitrage demandé d'abord :** accepter ou refuser explicitement **C** et sa
rétention locale sans cleanup PASS, limitée à l'absence d'inverses de nettoyage,
pas aux oracles DELETE requis. Sans cet accord, garde stricte inchangée. Si un
inverse reste requis, autoriser ou refuser l'étude détaillée de **A**, avec ses
trois écritures locales, l'interface privée de réservation/fermeture, le coût de
liaison atomique IAM et la limite SMTP explicites ; ou choisir **B** en autorisant
en plus les versions de contrats d'événements. Refuser les trois orientations
maintient le blocage de signup ; aucune option implicite « contourner la garde ».
Un accord sur cette orientation ne vaut ni Accept ADR ni GO d'implémentation :
il faut ensuite fixer formats/ports/migrations, autorisations, rétention, états
incertains, commandes QA et critères de preuve dans le contrat détaillé.

Les options A/B franchiraient le seuil ADR events/authz **0300** si retenues.
**0315 n'est pas disponible/réservé pour ce lot** : le parent signale la mémoire
`architecture/iam-signing-adr-0315` d'une proposition non intégrée sur autre branche.
Recouper les décisions concurrentes avant toute allocation. Aucun ID n'est alloué
ici. Proposition de titre pour A : « La fermeture causale d'un run
attesté précède l'inverse de ses effets asynchrones ». Aucune ADR/index/digest
n'est écrit dans ce delta de comparaison ; aucun choix n'est Accepted et aucune
cible ratifiée n'est modifiée. Le parent arbitre la réservation puis la rédaction
Proposed dans un lot documentaire distinct avant tout code.
Pour C, exception test-side locale sans contrat applicatif/persistance produit :
pas de nouvelle ADR proposée ici, mais accord utilisateur et amendement explicite
du contrat E2E requis avant admission du code. Le présent texte n'est pas cet accord.

## 5. Rayon et QA de la reprise éventuelle

Rayon actuel **contrat opérationnel/documentaire**, un seul fichier. Rayon d'une
solution future potentiellement **architectural / cross-module** : Ops
`Database.qualify_notifications_cleanup`, `Mesh.signup`, `cleanup`, `run_action`,
ports/commande/factory/writer Telegraph, migrations, consumer/setup, événements
IAM/Telegraph et projection Sentinel selon l'option retenue. Aucun de ces modules
n'est autorisé à changer par ce document ; monolithe, compose, SDK et FGA ne
deviennent pas tacitement un lot de développement.

Avant admission d'un lot : ownership production séparé du **test-engineer**,
contrat QA précis, régression ROUGE discriminante avant correction quand possible.
Le blocage environnemental actuel n'est pas ce ROUGE. Pour les options A/B,
dans les cibles existantes
`ops/tests/test_local_full_e2e.py`, puis si Rust autorisé
`services/Telegraph/tests/{outbox_tests,outbox_runtime_tests,notification_created_routing_test}.rs`,
exiger au minimum :

- tardif/replay après clôture et redémarrage, sans création résiduelle ;
- perte d'ACK avant/après commit et publication distante, sans adoption/doublon
  non recensé ; conflit d'ID/empreinte refusé ;
- handler concurrent intercalé à la fermeture, puis entre précondition et inverse ;
- ligne/delivery étrangère du même user conservée, dépendant inconnu bloquant ;
- crash/reprise à chaque transition, inverse déjà effectué avec ACK perdu,
  reçu terminal/stale/cible ou epoch remplacée refusés ;
- panne sortante et claim encore actif, projection Sentinel retardée/rejouée,
  transport disabled/NoOp jamais assimilé à traitement terminé ;
- réservation/commande étrangère ou contexte run forgé refusés ; manifeste
  incomplet, fermeture en conflit CAS et génération remplacée sans ACK terminal ;
  pour B, anciens messages sans contexte et version inconnue jamais adoptés ;
- qualification absente/stale : zéro signup et zéro SQL destructif, pas de faux
  `prepared=true`/cleanup PASS. Tests purement doublés ≠ preuve IT de ces races.

Commande Python candidate du **futur** delta Ops :
`python -m unittest discover -s ops/tests -p test_local_full_e2e.py`.
Les quatre commandes Python de la mission GitHub ne sont pas des validations
de ce constat et ne sont pas lancées. Toute future commande Rust ciblée doit être
définie **avant admission** selon les modules réellement autorisés, pas un
« tout cargo ». Windows natif, un Cargo, configuration 24 jobs inchangée ; IT
finales via harness réel avant E2E Kind, transport opt-in et cleanup des fixtures.
Revue secrets/trust indépendante si cette frontière change ; aucune extension
implicite du profil infra-verifier vers local-full.

## 6. Oracle exact du départ demandé — pas une preuve acquise

Après résolution de l'inverse **ou accord utilisateur explicite sur C**, puis
intégration du contrat/delta autorisé et qualification des producteurs :

1. GitHub **et GitLab effectifs**, cible/bail/UID/epoch et version courants
   attestés, PKI/HMAC conservés, contrôle infra requis réellement disponible.
2. `prepare-probes` strict réussi, ou préparation de rétention **distinctement
   déclarée après accord C**, sans autre précondition manquante et sans présenter
   l'inverse comme qualifié ; nouvelle validation des gardes du mode approuvé
   avant signup. Le reçu doit lier mode/accord, `run_id`, suite, HEAD, hashes source,
   cible/namespace UID et preuves courantes (structure `binding`, 1009–1026).
3. Pour **ce départ mesh**, au moins un **oracle métier requis réellement
   exécuté**, lié à ce run/HEAD/cible, constitue le témoin : `PASS`, ou `FAIL` avec
   `observed=true`, `completed=true`, `evidence=LIVE` selon `Results.case`
   (97–109). Par exemple, une réponse signup réellement reçue et comparée à
   l'attendu peut conclure FAIL : le parcours métier a commencé et la politique
   exige alors d'arrêter les corrections, pas de rendre signup vert. Conserver
   attendu/observé expurgés et leurs références privées. `account-created` et ses
   identités exactes sont exigés **si une création est effective**, pas comme
   condition universelle de lancement. Création possible mais ACK perdu =
   incertitude/résidu, pas adoption ni cleanup PASS. Un smoke/JWKS préalable ou
   un simple contrôle de précondition ne remplace pas l'oracle métier requis.
4. PID, import, `execution-started`, requête seulement tentée, 39 BLOCKED ou cas
   sans oracle complété ne prouvent pas ce lancement. Transport sans réponse
   observée, `FAIL / completed=false / evidence=NOT RUN`, BLOCKED et NOT RUN ne
   satisfont pas ce témoin. Le résultat métier peut être rouge sans que le départ
   réel de la suite soit un échec de lancement.
5. À cette frontière, arrêter les **corrections**, pas tuer le runner : laisser
   le parcours et son `finally/cleanup` du mode approuvé terminer (C : fermeture
   technique + reçu RETAINED, aucun inverse métier) ; rapporter exactement
   résidus/échecs, sans rerun correctif ni annonce de PASS global.

Conserver les **39 obligations mesh (12 positives/27 négatives)**, les **6
Apparatus**, et `SIMULATED_PROVIDER` : pas de preuve OAuth vendor réelle ni
raccord Apparatus inventé. Les cas non exécutés restent dans le dénominateur.
Le lancement réussi n'est pas synonyme de suite verte ou de cleanup déjà acquis.

## 7. Validation de ce livrable

Lectures canoniques/corps et références ciblées vérifiés ; 31 liens locaux
contrôlés, aucun absent ; `git diff --check` et contrôle staged sans erreur. Serena voit le canon
principal au HEAD ci-dessus ; les modifications non intégrées du worktree sont
vérifiées par outils fichiers, sans reconfiguration d'index. Aucun test applicatif,
signup, Docker/Kind/WSL ou lecture de secret nécessaire/exécuté pour A2.
La tentative de commit documentaire a déclenché le hook `.githooks` qui appelle
`cargo fmt` sans filtrer ce changement : échec Windows `os error 206` (nom de
fichier/extension trop long), commit refusé. Aucune commande Cargo n'a été
demandée directement, aucun build/test n'a été lancé ; hook non contourné et
aucune seconde tentative. Le fichier reste staged pour intégration par le parent.
Le parent conserve le ledger et publie l'état réel des gates ; ce document ne
constitue ni préparation réussie ni permission de lancer.
