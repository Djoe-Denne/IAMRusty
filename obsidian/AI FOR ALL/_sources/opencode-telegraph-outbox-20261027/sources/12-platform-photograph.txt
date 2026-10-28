# Événements, files et outbox

Les services publient des événements domaine (crates `*-events`). Le routage SQS est **par type d’événement**, pas un bus unique.

## Producteurs → files (dev)

| Producteur | Event types | File physique |
|---|---|---|
| IAMRusty | `user_signed_up`, `user_email_verified`, `password_reset_requested` | `telegraph-events` |
| Hive | `organization_*`, `member_joined`, `member_removed`, `member_roles_updated` | `sentinel-sync-events` |
| Manifesto | `project_created`, `project_visibility_changed`, `project_published`, `project_archived`, `component_*`, `member_*`, `permission_*` | `sentinel-sync-events` ; **`component_removed` fan-out aussi vers `lazaret-kv-events`** |
| Telegraph | `notification_created` | `sentinel-sync-events` |

Tests : mêmes clés, généralement préfixe `test-` (`test-telegraph-events`, `test-sentinel-sync-events`, `test-lazaret-kv-events`). La preuve Telegraph utilise `test-telegraph-outbound`, séparée de l'entrée IAM. Le principe est l'opt-in transport, pas un défaut uniforme des TOML : la configuration test Telegraph active historiquement les queues ; `services/Lazaret/config/development.toml` active aussi son consommateur `lazaret-kv-events`.

## Consommateurs

- **Telegraph** lit `telegraph-events` et mappe via `[queues.telegraph-events]` : `user_signed_up` / `password_reset_requested` → email ; `user_email_verified` → notification in-app.
- **sentinel-sync** lit `sentinel-sync-events` (ou sa config) et dispatch par préfixe `event_type` vers `HiveTranslator`, `ManifestoTranslator`, `IamTranslator`, `TelegraphTranslator`.
- **Manifesto** peut aussi *consommer* `component_status_changed` (apparatus) si la queue est réellement résolue.
- **Lazaret** lit `lazaret-kv-events` (tests : `test-lazaret-kv-events`) et purge le namespace KV du binding sur `component_removed` seulement.

Un événement sans bras translator : `None` — pas d’erreur, **pas de tuple**. Ajouter un event AuthZ-relevant sans mettre à jour `workers/sentinel-sync/src/translator/` = store FGA à la dérive.

## Traductions FGA (essentiel)

- Hive : rôles → `owner` / `admin` / `member` / `viewer` sur `organization`. Permission métier `write` → relation `member` ; `read` → `viewer`. Delete org : delete de toutes les relations connues pour owner + membres **portés par l’événement** (le translator ne relit pas la DB).
- Manifesto : membership + `project:{id}#viewer@user:*` si `visibility = public`.
- Telegraph : `notification:{id}#recipient@user:{user_id}` sur `NotificationCreated`.

## Outbox

`rustycog-outbox` : écrire l’événement dans la **même transaction** que l’agrégat, puis un poller publie sur SQS. Évite « persist OK / publish perdu ». Voir wiki `projects/rustycog/references/rustycog-outbox`.

`max_attempts = 0` sur `[command.retry]` **désactive** les retries (ce n’est pas « défaut infini »).

### Telegraph — état local et portée des preuves (2026-10-27)

Les deux créations enregistrent notification, livraison éventuelle et `NotificationCreated` dans une transaction writer PostgreSQL commune, sans publication directe post-commit. Une migration additionnelle ajoute l'outbox partagée ; la déployer avant le binaire, sans backfill historique. Le dispatcher exige un publisher Live et un transport concret non-NoOp : disabled/degraded suspend la publication sans faux `published`, tout en conservant HTTP. La reprise reconstruit l'application sur la même DB et conserve `event_id`/payload ; budget d'essais fini, un dispatcher actif par DB, ni fencing multi-worker ni exactly-once promis.

Le consumer, le dispatcher et le propriétaire HTTP adopté en standalone/monolithe partagent le budget local d'arrêt de 6 s (stop inclus). La jointure HTTP exhaustive est distincte de la sortie du parent ; Drop demande l'annulation mais ne prouve pas une jointure. Les preuves loopback et revues ciblées couvrent l'ownership HTTP et les corrections port 0 / CONNECT HTTP/2, pas toute la plateforme ou tous les upgrades. Le SDK testé `d60a88b2f2f499768a807941a1b4c80c9a366b3f` est publié sur la branche dédiée `mission-telegraph-ac7-zlfadf`, vérifié par `ls-remote` ; la livraison consomme ce SHA exact via le gitlink, sans modifier la branche principale SDK.

La preuve verticale historique `t3-exp3-6125e6e3d1d54e21` (1 PASS, 0 ignoré, 6 filtrés) corrèle SQL et vrais bytes SQS, appelle le vrai SyncEventHandler/OpenFGA sans droit préchargé, rejoue avec ledger neuf, vérifie tuple unique et HTTP 403 sans mutation / 200 destinataire, au nominal et après panne sortante. Elle est calibrée : visibilité IAM INBOUND initiale 30 s puis 5 s avant la première app, deadline SQL inchangée à 25 s. Les deux timeouts historiques à visibilité 30 s restent inexpliqués : ni correction SQS démontrée, ni SLA 5 s, ni annulation distante garantie.

Les deux IT adaptées au propriétaire HTTP ont été exécutées sur AI1f2dd30/SDKd60 : NoOp/reconstruction 1 PASS, vertical 1 FAILED au contrôle `next_attempt_at > now` (rapport97). Le diagnostic105 mesure une programmation correcte à environ5s et une vérification trop tardive, sans inventer les valeurs exactes de97. La correction temporelle test-only106 est intégrée et revue PASS borné. Sa validation unique échoue plus loin : après arrêt, restauration de destination et reconstruction, PUBLISHED n'est pas observé sous25s (rapport109), avant la fin reprise/replay/autorisation du cas panne.

Le diagnostic passif110 réussit ensuite le scénario complet sur source instrumentée isolée, depuis un état failed sans verrou. **La course109 n'est pas observée et sa cause reste non résolue** ; ce vert ne remplace pas le rouge historique. L'instrumentation reste archivée, hors source livrée. Voir la [photographie et les limites du contrat](../contracts/telegraph-notification-outbox.md#10-photographie-locale-et-preuves-bornées--2026-10-27). Ces résultats et blocages sont historiques ; ADR0313 reste Proposed et ADR0301 Partial.

Le témoin contrôlé117 démontre la conservation du claim SQL après annulation locale, sans nouvelle prise en charge pendant la fenêtre observée avant expiration. Le témoin complémentaire122, après nettoyage et correction du seul identifiant de lancement, réussit sur le run `post-expiry122-6412a9c40ae64bf4` : **1 PASS, 0 échec**,15 cycles avant expiration puis1 après, nouveau claim, transport SQS réel et même événement reçu, comptes métier inchangés. Les observations terminent30,244s après reconstruction avant l'échéance fixe54,990s. Ce résultat prouve la reprise contrôlée après expiration, pas la cause109 ni la verticale finale. Fin de campagne :0 fixture possédée active,3 retirées,1 SMTP arrêté et2 volumes opaques conservés ASK ; préexistants préservés.

L'approche retenue conserve le produit : lease SQL30s, poll2s, backoff5s, maximum20 essais. La preuve cible l'éligibilité persistée avec une échéance fixe bornée, sans déverrouillage anticipé, raccourcissement de lease ou nouveau drain. La lease SQL est distincte de la visibilité SQS5s du scénario IAM. Arrêt local6s ne signifie pas annulation d'un effet distant. Aucun exactly-once, fencing multi-worker, déduplication IAM ou backfill n'est promis.

La validation verticale finale autorisée `vertical134-06bd9d7e772c4370` réussit sur la source test `80cce027` revue indépendamment : **1 PASS, 0 échec, 0 ignoré, 6 filtrés**,25,39s, nominal et panne/reconstruction. Elle conserve vrais bytes SQS, handler/OpenFGA sans grant préalable, replay avec ledger neuf, tuple unique,403 sans mutation et200 destinataire. Dans ce run, l'intention était déjà éligible : R0, deadline fixe25s reconstruction incluse, observations terminées en266736µs. Le cas de lease retenue est couvert séparément par le composant122, pas artificiellement revendiqué dans cette verticale. Le rouge109 reste inexpliqué et conservé.

Fin de validation finale :0 fixture possédée active ;3 retirées,1 SMTP Exited(2) et2 volumes opaques conservés ASK ;15 préexistants intacts. Clippy strict préexistant non vert, Sonar non exécuté, CI distante et toutes les branches d'erreur de startup non revendiquées : le succès ciblé ne vaut pas quitus qualité global. Rapports139/140 et ledger de mission conservent preuves et réserves.

## Fan-out

SQS est 1:1. Pour N consommateurs, déclarer N files dans `[queue.queues]` pour le même `event_type`, ou un worker dédié. Ne pas compter sur une file unique partagée.

Manifesto `component_removed` fan-out : `sentinel-sync-events` **et** `lazaret-kv-events` (tests : `test-sentinel-sync-events` **et** `test-lazaret-kv-events`). Les autres event types Manifesto ne ciblent **pas** la file Lazaret.

## Suite

- Skill : [`.agents/skills/rustycog/references/using-rustycog-events.md`](../../.agents/skills/rustycog/references/using-rustycog-events.md)
- [authz-openfga.md](authz-openfga.md)
- [../services/sentinel-sync.md](../services/sentinel-sync.md)
