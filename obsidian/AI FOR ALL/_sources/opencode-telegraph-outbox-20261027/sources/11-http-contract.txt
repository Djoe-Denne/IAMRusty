# Contrat d'implémentation — propriétaire HTTP et arrêt AC7

- Version 1 — 2026-10-26 ; périmètre autorisé par rapport 53.
- Canon décisionnel : [ADR 0503 Proposed / Unimplemented](../adr/0503-http-runtime-owned-shutdown.md).
- Contrat parent : [Telegraph notification outbox](telegraph-notification-outbox.md), §5 et AC7.
- Baseline historique du défaut/E1 : aa400f38608e92a34c463f1f89b02168f8364440 / SDK129dcc1575e7dd59b4c7935de0b3794dbb7e7d07.
- HEAD documentaire après intégration E1 : b291eee3b7537bfa02cebb5df844d5b4ca2019bf ; C-1 ouvert, aucun correctif SDK livré.

## 1. Autorité, preuve acquise et gate

Le rapport 53 autorise le correctif HTTP SDK et ses consommateurs bornés, pas l'Accept d'une ADR. Références au dépôt principal : `.cursor/qa-plans/20261026-http-ac7-c1.md`, `.cursor/review-briefings/20261026T1704Z-correctness-telegraph-outbox-t4-aa400f3.md`, rapports 53/55/57 de `.opencode/missions/team_mvo1md2x_0000_zlfadf/reports/`.

**E1 rouge réel est acquis et intégré** : `http_shutdown_destroys_inflight_handler_before_return_without_late_mutation`, cible `services/Telegraph/tests/outbox_runtime_tests.rs`, source blob `221abe3fc7a1b751c8a1d574b422f767ee68abca`. Exécution baseline : 0 PASS / 1 FAILED / 0 ignored / 4 filtered, exit 101 ; compilation réussie. Shutdown Ok, parent terminé, RAII handler 0 au retour, puis mutation tardive 1 après libération. RAII/EOF/socket nettoyés avant assertion rouge. Aucun Docker, correctif SDK ou preuve du monolithe complet n'en est déduit.

E1 et ce contrat/API doivent être intégrés avant implémentation SDK ou publication de consommateurs. E1 est déjà intégré dans b291eee ; **ne pas le qualifier d'attendu ni le relancer pour écrire ce contrat**. Conserver logs et provenance historiques. Le contrat reste l'entrée documentaire à intégrer.

AC1–AC7, replay Sentinel à ledger neuf, calibration INBOUND5s et rouges historiques restent inchangés. Les deux docs T5 staged ailleurs sont hors propriété ; ne pas les écraser ni les intégrer implicitement.

## 2. Surface autorisée

SDK source isolée descendante de 129dcc1 :

- `rustycog-http/src/builder.rs`, `src/lib.rs` et module privé `src/owned_server.rs` si nécessaire.
- `rustycog-http/src/tls.rs` seulement pour raccorder minimalement l'acceptor existant à l'ownership, sans refonte TLS/authentification.
- **Cargo.toml racine du SDK unifié**, pas de Cargo.toml HTTP séparé ni nouvelle crate. Dépendances directes optionnelles seulement si indispensables au contrôle de l'exécuteur Hyper existant, sous feature HTTP ; pas d'upgrade gratuit, backend ou feature globale. Locks affectés justifiés et revus.
- Tests colocalisés du propriétaire et cibles HTTP existantes, notamment `mtls_client_auth`.

AIForAll :

- `services/Telegraph/setup/src/{app,runtime}.rs` ; `runtime/monolith/src/runtime.rs`.
- `services/Telegraph/tests/fixtures/outbox_runtime.rs`, `tests/outbox_runtime_tests.rs` et modules de tests runtime existants, pour raccordement/preuves HTTP seulement.
- Gitlink `rustycog` et locks nécessaires après publication du SHA exact revu.

Pas de nouveau harness/service/backend, changement events/outbox/Sentinel, migration SQL ou protocole métier. Préserver routes/préfixes, ServerConfig, ConnectInfo, TLS-only/dual-bind, mTLS fail-closed et protocoles existants. Le monolithe adopte le propriétaire de son listener commun ; les autres standalones ne migrent pas.

## 3. API additive minimale

Exports sous `rustycog::http`. Signatures contractuelles, non encore livrées :

```rust
pub async fn start_router_owned(
    app: axum::Router,
    config: ServerConfig,
) -> anyhow::Result<OwnedHttpServer>;

impl HttpShutdownBudget {
    pub fn new(
        grace_at: tokio::time::Instant,
        deadline: tokio::time::Instant,
    ) -> anyhow::Result<Self>;
}

impl OwnedHttpServer {
    pub fn local_addrs(&self) -> Vec<std::net::SocketAddr>;
    pub async fn wait_for_server_exit(&mut self) -> anyhow::Result<()>;
    pub fn request_shutdown(&mut self, budget: HttpShutdownBudget);
    pub async fn join(&mut self) -> anyhow::Result<HttpShutdownReport>;
}

pub struct HttpShutdownReport {
    pub forced: bool,
}
```

- Le propriétaire n'est pas clonable et reste dans le caller. Il conserve les moyens d'observer/annuler ses descendants même si le futur serveur interne termine ou panique.
- `start_router_owned` valide TLS/mTLS et prépare les listeners demandés avant admission ; tout travail démarré est enregistré. Échec normal de startup : fermer les ressources partielles et observer le travail possédé, sans listener sibling oublié. Annulation de startup : guard/Drop d'annulation, sans prétendre attendre dans Drop.
- `local_addrs` rend les adresses réellement liées, dont port 0 ; en dual-bind ordre HTTP puis HTTPS.
- `wait_for_server_exit` observe le futur serveur/coordinateur, **pas la terminaison exhaustive**. Erreur/panic/sortie déjà terminée est notifiée sans attendre un drain caché de six secondes. Annuler ce futur emprunté conserve l'objet et ses résultats ; aucune consommation partielle ne les perd.
- `request_shutdown` est synchrone, sans attente réseau : fermeture logique de l'admission et transmission des échéances. Appels répétés idempotents, échéances seulement raccourcies, jamais prolongées. Serveur parent déjà terminé : descendants toujours joignables. Le constructeur rejette grace_at > deadline ; une échéance passée n'ouvre aucun nouveau délai.
- `join` est la seule attestation de terminaison complète après demande d'arrêt : coordinateur ET descendants observés/joints, même si le coordinateur a déjà échoué. Sans budget demandé : erreur d'usage, pas attente infinie. Annuler ce futur emprunté conserve les enregistrements pour reprise de jointure ou Drop. Un nouvel appel après jointure conserve le résultat terminal sans double-poll d'un handle consommé.
- `forced=false` : drain complet dans la grâce. `forced=true` : annulation planifiée nécessaire ET terminaison observée. Le caller conserve ce diagnostic ; annuler une requête ne la transforme jamais en succès HTTP.
- Préserver `serve_router(app, config) -> anyhow::Result<()>`, `RouteBuilder::build` et leurs consommateurs. Leur abandon n'acquiert pas rétroactivement cette nouvelle garantie.

## 4. Ownership et protocoles

1. Posséder accept loop, connexions, requêtes/extracteurs/middlewares/handlers async, bodies entrants/sortants et streams, handshakes pertinents et tâches d'exécuteur HTTP/2. Joindre au diff l'inventaire des protocoles/features réellement supportés ; ne pas exclure ou désactiver silencieusement un upgrade déjà supporté.
2. Enregistrer un descendant avant son travail non suivi. La fermeture concurrente de l'admission ne laisse aucun enfant hors groupe, y compris les spawns indirects de l'exécuteur.
3. L'ownership de secours ne disparaît pas avec le coordinateur. Récolter les tâches terminées pendant le service, sans accumulation indéfinie de handles finis.
4. Stop : arrêter l'admission de nouvelles connexions/requêtes ; une course déjà admise reste possédée. Laisser terminer le travail admis dans la grâce restante.
5. À grace_at, annuler réellement les futures restantes, y compris handlers/bodies/exécuteur, puis observer leur fin avant deadline. Signal graceful, compteur nul ou jointure parent ne remplacent pas cette preuve.
6. Réutiliser la pile publique Axum/Hyper/rustls existante, sans fork/vendor, backend neuf, runtime/thread dédié ou driver parallèle dans setup/monolithe. Middleware autorisé comme mécanique, jamais comme faux quitus d'ownership.

Registre, notifications et exécuteur concrets restent des choix privés. Si les invariants ne sont pas réalisables dans cette surface, revenir au lead avant extension ou réduction de garantie.

## 5. Caller, échéances, erreurs et Drop

Le caller :

1. Conserve le propriétaire HTTP et ses guards avant les attentes fallibles ; une erreur de startup n'abandonne pas des tâches métier déjà démarrées.
2. Supervise signal externe, tâches métier et sortie serveur. Une sortie HTTP non demandée, même Ok, est inattendue.
3. Calcule t0 avant tout stop, grâce t0+5s et deadline t0+6s, une seule fois pour tous les groupes.
4. Demande immédiatement l'arrêt HTTP et déclenche les stops métier sans attendre un drain HTTP préalable. Joint les groupes sous la même deadline. Le Vec de handles métier reste compatible ; HTTP est une ressource possédée distincte.
5. Adapte le vrai helper partagé : à cinq secondes annuler le travail restant, puis observer jusqu'à la deadline. Ne pas aborter seulement le coordinateur HTTP ni interrompre sa jointure à la fin de la grâce en abandonnant sa phase forcée.
6. Consomme résultats déjà finis, erreurs et panics ; conserve erreur primaire et diagnostics de cleanup.

Les déconnexions client ordinaires gardent leur politique existante ; ne pas en faire toutes des pannes du host. Une panic de tâche possédée non récupérée est contextualisée, notifiée et propagée après cleanup. Une panic déjà convertie par le middleware en réponse conserve cette politique. Cancellation planifiée attendue seulement après force demandée ; bind/TLS/erreur terminale ne sont pas masqués par un stop réussi.

Un dépassement ou une terminaison non observée reste une erreur avec diagnostics et demande d'annulation maintenue, jamais un rapport d'arrêt complet. Six secondes n'est ni une durée par handle/listener ni un nouveau SLA.

**Drop** ferme l'admission et ordonne l'annulation, sans await, async join ou blocage de thread. Son retour ne prouve pas que les destructeurs des autres tâches ont tourné ; il n'émet aucun succès. Un caller encore actif conserve l'objet et attend join. Parent annulé/paniqué : guard/Drop d'annulation ; tests sur Tokio encore actif avec témoins externes de destruction effective, pas seulement parent fini.

Exclusions inchangées : sync non coopératif/boucle CPU sans yield, spawn_blocking, kill processus/OS, terminaison d'effets réseau distants. Un handler/body async local n'en fait pas partie. Aucune promesse d'annuler un commit distant déjà accepté.

## 6. Adoption et tests ciblés

Standalone : vrai `create_prefixed_router` puis propriétaire SDK. Monolithe : `compose_routes` et listener commun, sans appeler `TelegraphApp::run()`. Même helper de cleanup en production et tests, pas copie test-only. `RunningApp` migre vers cette surface, sans select-stop abandonnant axum::serve.

Le rouge E1 baseline et ses logs restent immuables. Pour le vert, migrer seulement son raccordement vers le chemin réellement adopté, sans affaiblir RAII/mutation/délai. Ne pas ignorer E1 ni imposer au legacy une nouvelle garantie non adoptée.

Compléments ciblés, non exécutés par ce document :

- Drain gracieux d'une vraie requête entrée ; handler bloqué nécessitant force : RAII détruit avant retour, aucune mutation après libération, budget stop inclus.
- Body entrant lent, stream sortant bloqué après headers, keep-alive/admission après stop ; témoins de destruction des propriétaires concernés.
- Handshake TLS suspendu et tâches HTTP/2 des protocoles supportés ; pas déduction depuis le seul compteur de connexions. Réutiliser clients/certs/fixtures existants, sans Docker.
- Erreur/panic coordinateur avec descendant actif ; bind/TLS/échec partiel dual-bind ; Drop/parent annulé ; cancellation d'un join emprunté. Observer destruction et diagnostics sans prétendre qu'un Drop a attendu.
- Préserver mTLS fail-closed, dual-bind et protocoles sur l'API propriétaire ; régressions legacy touchées, dont cible SDK existante `mtls_client_auth`.
- Vert des mêmes invariants standalone et monolithe via vrais chemins/helpers adoptés. Routeur local acceptable sans construire la plateforme ; serveur simplement pending insuffisant.

E1 : socket TCP loopback réelle, entrée handler synchronisée, vraie surface serveur et shutdown ; mémoriser les observations, libérer/joindre client/handler/socket avant assertion susceptible d'échouer. Compilation/config/port en échec n'est pas un rouge fonctionnel. Ce rouge est déjà acquis, pas à réexécuter pour ce contrat.

Cibles : `telegraph-service --test outbox_runtime_tests`, tests lib runtime `telegraph-setup`, tests runtime du manifeste monolithe, tests propriétaire SDK et `mtls_client_auth` via **rustycog/Cargo.toml racine**. Le lead consigne filtres/noms réels, nombre exécuté >=1/0ignored et logs ; pas tout cargo ni recampagne outbox/FGA/SQS.

Futures validations : Windows natif, un seul Cargo, 24 jobs configurés sans override ; Context Mode et logs bruts privés. Relecture indépendante bornée T4 après diff intégré ; spécialiste concurrence seulement si risque distinct utile. Aucun test vert/Accept ADR/PASS global déduit de ce contrat.

## 7. Publication et pin

Source SDK isolée autorisée, base 129dcc1 et propriété inventoriées ; préserver changements utilisateur, pas commit/stash/reset automatique ni édition perdue du submodule.

Après tests ciblés et revue, publier le SHA exact revu sur le remote SDK **avant** gitlink AIForAll. Conserver SHA/parent, logs du SHA testé et disponibilité remote ; pas HEAD mouvant, path sibling ou copie vendor. Accès refusé = blocage.

Le lead intègre gitlink mode160000 et consommateurs autorisés, recoupe locks/sources/preuves puis demande la relecture ciblée. Aucun bump incident. Aucune publication ou opération runtime effectuée par l'architecte.

## 8. État et décisions fixées

Fixés : API additive, propriétaire caller, observation parent distincte de jointure complète, échéances communes, descendants/protocoles conservés, Drop sans async join, surface/adoption/pin bornés. Choix privés : registre/exécuteur, notifications, diagnostics et noms de tests.

Tout besoin d'API générique, nouveau backend/harness/cible, désactivation de protocole ou oracle réduit revient au lead avant changement.

E1 rouge exécuté et intégré dans b291eee ; aucun correctif SDK livré ; C-1/AC7 restent ouverts. ADR0503 et0313 Proposed, ADR0301 Partial. Sources : finding T4 final, QA C-1, rapports53/55/57 ; SDK unifié builder.rs236–299 ; callers setup/app.rs165–195, setup/runtime.rs49–103, monolith/runtime.rs95–127 et fixture outbox_runtime.rs33–74.
