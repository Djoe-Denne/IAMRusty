# Contrat proposé — durée complète des tests IAM

Date : 2026-10-06. Analyse statique, HEAD de référence `9c64841` + corrections IAM non committées du worker actif. Aucun Cargo, test, Docker/WSL/Kind, nettoyage, modification applicative ou submodule exécuté ici. Les chiffres CI/runtime ci-dessous sont des preuves transmises par le parent, non des mesures exécutées par cette feuille.

## 1. Sujet et décision recommandée

Jalon : fermeture IAM AuthN/IT, transversal harness SDK ; hors Apparatus P0–P6.

**Rechercher et instrumenter les coûts, traiter le calcul répété IAM et le cycle de vie SDK séparément ; agréger seulement si les mesures le justifient.** Objectif cinq minutes, pas promesse. Valider le candidat partagé sur IAM avant propagation aux autres services.

Priorité des options :

1. **Instrumentation SDK + réduction locale des transformations JWKS redondantes + mutualisation du seul matériau RSA de fixture neutre** : coût borné, réutilisable pour la mesure, sans nouveau stockage ni protocole. Les deux optimisations IAM sont conditionnées à l'équivalence et au temps de phase mesuré.
2. **Fin de processus drainée : arrêt gracieux des fixtures réellement possédées via runner parent/outil partagé SDK** : supprime l'accumulation inactive sans réutilisation interprocessus ; gain CPU/temps inconnu. Pas de finaliseur async caché ni de changement de `Drop`.
3. **Agrégation HTTP IAM, transports séparés** : réduit les initialisations de processus/conteneurs, pas les migrations par fixture, générations RSA par listener ni les boucles de calcul. Facultative après mesure, sans parallélisme supplémentaire initial.
4. **Broker interprocessus / DB template-clone / reset accéléré** : non retenus à ce stade. Ils introduisent autorité, leases, isolation et restauration coûteuses à spécifier. `with_reuse` seul n'est pas ce contrat.

## 2. Baselines et conclusions limitées

- Référence utilisateur : https://github.com/Djoe-Denne/IAMRusty/actions/runs/35511266082/job/106080033961 ; 20/09, SHA `c465fb7815f5408253c89c84d7c649299e691cb7`, succès. Même job Coverage-integration IAM, ubuntu-latest, même sélection des sept packages et `cargo llvm-cov --locked --no-report --no-fail-fast --tests`. Compilation **5m00** ; première cible 12:50:33.270 → dernier résultat 12:54:34.523 = **4m01.254** ; somme libtest 240.90s, 26 externes + 7 libs, 282 pass / 0 fail / 1 ignored.
- Run `37358153415`, SHA `9c64841` : compilation **5m03**, exécution résultats **21m08**, 47 externes + 7 libs, 499 pass / 62 fail / 1 ignored. La sélection a grandi à 561 tests exécutés : comparaison non équivalente, pas preuve que les conteneurs expliquent tout. La référence rapide avait déjà 26 binaires externes.
- Snapshot parent 11:30:44Z : le run local final terminé avait 54 résultats, 51 OK / 3 FAILED (`internal_provider_token`, `oauth_cleanup_lifecycle`, `user`), 36 PostgreSQL créés puis arrêtés ; aucun ancien encore running. Un autre run avait `signing_admission` actif depuis environ 3m15, ~96% d'un cœur. Ce snapshot privilégie l'étude du chemin CPU, sans profiler une fonction précise ni vérifier l'état courant.

## 3. Sources et contradictions

Canon : ADR-0200 serveur/DB réels et serialisation des singletons ; ADR-0201 mocks HTTP sortants seulement, OpenFGA réel ; ADR-0202 queues opt-in ; ADR-0310 admission atomique, DTO exact et bornes RSA/JWKS ; ADR-0412 transaction OAuth et supervision. Briefs : `.cursor/review-briefings/{INDEX.md,20261005-session-standards.md,20261003-review-fixes-baseline-f060d47.md,20261004-security-sdk-fixtures-prepublish.md,20261004T1339Z-rust-perf-admission-fixture-final-5348a63.md,2026-10-05T1719Z-tests-rsa8192-env-probe-87a6a10.md}`. Handbook : `docs/guides/tests-integration.md`.

Code statique déterminant :

- `rustycog/rustycog-testing/src/common/database.rs` : `TestDatabase::new` crée un pool et appelle down/up à chaque fixture ; singleton `OnceLock` = **un processus**, pas une suite Cargo. Retention statique, `cleanup_container_on_drop=false`, atexit informatif. Ancien `ca2e35f` également process-local ; stop/rm global par nom précédait une recréation, pas un rattachement. Le correctif ownership `c57eee9` consommé par `29c704f` doit rester.
- `.../fixture_runtime.rs` : identité run/PID/attempt, ID retourné à la création ; `OwnedContainer::rm`, `take_unshared`, superviseur et `join_fixture_creations`. La dernière est une réconciliation non bloquante des créations orphelines, **pas** une preuve de libération des consommateurs.
- `.../openfga_testcontainer.rs::TestOpenFga::new` : conteneur process-global mais **store/modèle frais par fixture**, env publiées. `.../sqs_testcontainer.rs::TestSqs::new` : singleton LocalStack, client/readiness/queues reprovisionnés, queues nommées partagées et env globales. Mutualiser ne doit pas fusionner leurs isolations.
- `services/IAMRusty/tests/common.rs::start_owned_listener` (dirty, ~298–330) génère un RSA2048 pour chaque listener par défaut ; `prepare_primary_fixture` refuse une migration sous un listener vivant ; `start_owned_listener_with_keys` construit un app/registry/JWKS/port instance-local. Préserver cette correction fonctionnelle.
- `tests/signing_admission.rs::cross_org_last_budget_slot_is_atomic_platform_reserve_and_publisher_survive_rejection` (~314–334) remplit jusqu'à 720 admissions, avec snapshot + accounting complet à chaque tour. `domain/src/entity/signing_key.rs::{publication_usage,reserved_entry_bytes}` refait l'accounting complet ; `domain/src/entity/token.rs::JwkSet::from_registry_keys_checked` parse puis reconstruit via `from_registry_keys`. Sur matériau valide : jusqu'à **huit conversions PEM/JWK par clé** dans un `publication_usage` (DTO actuel + trois variantes de statut, chacun validé puis reconstruit). Le remplissage cumule un travail quadratique ; fusionner les fichiers ne le supprime pas. Ce coût est établi statiquement, son poids temporel reste à mesurer.

Écarts documentaires : le skill `creating-testcontainer-fixtures` recommande encore eviction par nom/`rm -f` et évoque OpenFGA mock : contraire à ownership/ADR-0201, **ne pas suivre**. Wiki `obsidian/AI FOR ALL/projects/iamrusty/references/iamrusty-testing-and-fixtures.md` décrit encore un boot HS256 sans PEM ; la fixture actuelle exige du RS256 réel. Aucun précédent d'agrégation explicite dans les manifests Hive/Manifesto/Telegraph inspectés. GrepAI sémantique et Serena ont fonctionné ; le graphe GrepAI de noms génériques `new/write` comporte de fausses résolutions : corps Serena autoritatifs ici.

## 4. Documentation officielle versionnée

`Cargo.lock:6518–6547` relie `rustycog-framework` à **testcontainers 0.24.0** (`Cargo.lock:7948`) ; 0.23.3 existe aussi pour d'autres dépendances. Déclaration SDK : `rustycog/Cargo.toml:217`, pas un manifeste autonome rustycog-testing.

- https://raw.githubusercontent.com/testcontainers/testcontainers-rs/0.24.0/README.md
- https://docs.rs/testcontainers/0.24.0/testcontainers/
- https://raw.githubusercontent.com/testcontainers/testcontainers-rs/0.24.0/testcontainers/Cargo.toml
- https://raw.githubusercontent.com/testcontainers/testcontainers-rs/0.24.0/testcontainers/src/core/image/image_ext.rs (160–168)
- https://raw.githubusercontent.com/testcontainers/testcontainers-rs/0.24.0/testcontainers/src/core/containers/async_container.rs (407–442)
- https://raw.githubusercontent.com/testcontainers/testcontainers-rs/0.24.0/testcontainers/src/runners/async_runner.rs (95–133)
- https://raw.githubusercontent.com/testcontainers/testcontainers-rs/0.24.0/testcontainers/src/watchdog.rs (1–46)

L'API prévoit removal-on-Drop ; un singleton statique ne sort pas normalement de scope à chaque test/process exit. `TESTCONTAINERS_COMMAND=keep` et la feature expérimentale `reusable-containers` retiennent les conteneurs ; `Always/CurrentSession` sautent leur reaping. Le runner retrouve un conteneur selon nom/network/labels, pas selon notre preuve de création ni un lease applicatif. Il ne fournit pas l'isolation des schémas/stores/queues. Le watchdog est optionnel, désactivé par défaut, traite SIGTERM/INT/QUIT et stop+rm ; **ce n'est pas une garantie Ryuk livrée ni un teardown normal de nos singletons**. Pas d'activation automatique, pas de bump de dépendance nécessaire pour la première tranche.

## 5. Blast radius et invariants

Cross-file IAM + module SDK de tests ; pas de frontière service, API HTTP, schema ou migration applicative à changer. Chemins candidats : common/signing_admission, domain/entity/{signing_key,token}, SDK common/{database,fixture_runtime,openfga_testcontainer,sqs_testcontainer}. Manifest/test-layout/runner seulement en tranche conditionnelle ; aucun compose/Kind/monolithe à migrer.

Invariants : serveur et PostgreSQL réels ; migrations et isolation par cas ; transaction/pool empruntés, pas de `db.clone()` ; stores OpenFGA frais/deny réel ; queues désactivées hors tests transport et destination/fallback inchangés ; locks serial globaux ; apps, registries, caches JWKS, epochs, issuer/org bindings et limiteurs ne se partagent pas entre scénarios. OAuth `execute_once` reste au-plus-une-exécution (ne pas inventer une garantie exactly-once), preuves non clonables, races SQL et TLS/IdP vérifiés intacts. RSA2048..8192, budgets ratifiés, erreurs opaques, secret RSA8192 obligatoire et taille vérifiée : aucun early-return/ignore nouveau.

## 6. Contrat implementer, tranches bornées

**A — mesure sans suite complète supplémentaire.** Attendre le checkpoint et la fin des modifications fonctionnelles/leases du parent. Capturer SHA root/SDK + dirty hash et manifeste exhaustif cible/test/feature/ignored ; `--list` après compilation brokerisée, pas de compteur de fichiers comme preuve. Instrumentation opt-in monotone, non secrète, dans SDK + helper IAM : acquisition/start/readiness, pool, down/up, construction app/bootstrap RSA/probe, action, accounting/admission, shutdown/join. Compter appels/clés converties et créations, sans log de PEM, URL credentialée, cookie/token. Ne pas confondre temps inclusifs imbriqués avec somme murale. Exploiter les runs existants ; probes ciblées représentatives sous un seul bail Cargo, jamais une succession de full runs de tuning.

**B — candidat CPU IAM sous contrat existant.** Construire une fois le DTO strictement validé de chaque clé, conserver exactitude complète/duplicated kid/algorithm/issuer/trust/status/PEM, réutiliser ses champs publics pour les variantes de statut et sérialisation exacte ; pas de cache global ni de nouvelle comptabilité persistée. Interfaces, erreurs, verrou writer et snapshot DB cohérent inchangés. Tester contre une référence indépendante de l'ancien serializer : bytes exacts, Unicode/escaping, Pending/Active/Retiring/Revoked, TTL/skew, invalid/duplicate keys, RSA boundaries, plafonds/admissions/refus sans effets. **Ne pas remplacer les admissions du remplissage par SQL brut ni abaisser les quotas.** Une réduction des snapshots du test n'est autorisée qu'avec frontier initial/final vérifiée et mêmes admissions/assertions, jamais un compteur auto-confirmant.

Pour le RSA de boot neutre : une paire valide immuable/temporaire générée une fois par processus (pas un cache de provider/registry/app) peut être partagée, `kid` frais par instance. Les tests changement de matériau/mismatch/rotation utilisent des paires réellement distinctes ; auditer leurs callers avant substitution. Conserver parse/probe/validation réelle au boot. Aucun partage automatique de clients/pools async entre runtimes Tokio distincts.

**C — candidat SDK de cycle de vie.** Première livraison réutilisable : timings/provenance et rapport des fixtures terminales. Candidat d'arrêt via runner explicite seulement : enfant fini + producteurs/consommateurs quiescents prouvés, ledger run/PID/attempt et ID de création concordants, bail libéré ; stop gracieux par ID, observer/loguer `stopped`, conserver `no/ask/UNKNOWN` pour arbitrage. Pas de rm/prune/name-adoption, pas d'arrêt d'un singleton entre deux tests consommateurs ni destructor async. Si une nouvelle autorité runner/lease/API publique devient nécessaire, revenir à l'architecte et à l'humain avant de la figer ; pas de solution interprocessus déguisée en `with_reuse`.

**D — agrégation conditionnelle.** Si start/retention de processus domine encore : `iam-service` avec `autotests=false`, un `tests/integration.rs` et modules de cas, quatre cibles explicites : `integration`, `sqs_event_routing_tests`, `signup_sqs`, `signup_kafka`. Préserver cfg feature/ignored actuels ; le helper common cesse seulement d'être une cible vide. Un seul module common/fixtures/utils/support par binaire, imports `crate::...`, locks serial communs, migrations après drain ; ne pas dupliquer les statics de leases via plusieurs `mod common`. Autres crates conservées. Aucun parallélisme supplémentaire initial ; ne pas régler le problème par plus de CPU. Mapper chaque ancien `(package,target,test)` vers le nouveau, auditer path/cfg et lcov. Le scan statique des helpers fixtures/utils/support n'a trouvé aucun test déclaré, à vérifier au manifeste effectif.

## 7. Validation et propagation

Sélection principale exacte, sans filtre ni réduction :

`cargo llvm-cov --package iam-service --package iam-domain --package iam-application --package iam-infra --package iam-http_server --package iam-setup --package iam-configuration --locked --no-report --no-fail-fast --tests`

Compile `--locked --all-targets` de cette matrice en Docker, separately timed ; sélecteurs purs ciblés puis IT touchées de layout/lifecycle/signing/OAuth sous bail. Revue correctness + test + security ; rust-perf pour transformations/ownership/SDK. Suivre la checklist CI exacte du standard de session, pas « tout cargo ». SDK : gates ciblés verrouillés + tests purs/reviews → publication `main` → checkout détaché → bump du seul gitlink parent ; aucune modification silencieuse du checkout consommé. D'abord IAM ; Hive/Manifesto/Telegraph seulement après gain IAM démontré et revue de leurs stores/env/transport/SMTP, chacune avec sa sélection intacte.

**Un seul benchmark intégré final**, après fixes prêts et slots/leases réconciliés. Environnement/CPU/RAM effectifs documentés, pas supposés ; mêmes scénarios actuels, features, instrumentation llvm-cov et politique de cleanup. Séparer `T_compile`, `T_results` (première cible → dernier résultat, comparable aux baselines), `T_complete` (première exécution → toutes cibles terminées + drain/arrêts possédés vérifiés), `T_report` lcov. Tout exclu est publié ; aucune phase nécessaire cachée. Cible `T_complete <= 300s`, sinon résultat honnête avec gains et coût résiduel, pas échec compensé par skip/hardware. Succès fonctionnel = tous scénarios actuels passent, multiplicité justifiée, ignored/features identiques, assertions/coverage mappées et aucun bail orphelin/ressource étrangère touchée. Une ancienne suite à 282 pass ne valide pas la nouvelle.

## 8. ADR et escalade

**Aucune ADR maintenant** : instrumentation, conversion DTO équivalente, matériau de fixture neutre et layout restent réversibles et bornés par ADR0200–0202/0310 ; cinq minutes reste objectif non ratifié comme SLO. Pas de draft/digest écrit ici. **0203** est le prochain libre observé, à revérifier si une session/broker interprocessus, template/reset DB ou nouvelle autorité de lifecycle est réellement choisie : décision séparée, Proposed + digest/pointeur obligatoires, Accept humain uniquement.

Mandat reçu : rechercher/améliorer sans affaiblir, SDK réutilisable autorisé en principe. Arbitrages futurs : nouvelle frontière/autorité SDK, réutilisation interprocessus, changement de reset/migrations, critères dépassant le contrat ; aucune restauration de l'ancien nettoyage global. Le parent choisit le paquet B/C/D à la lumière des timers, pas une promesse basée sur le nombre de fichiers.
## Addendum 2026-10-06 — blocker fonctionnel JWKS, AVANT performance

### Décision bornée / ADR

Retenir un **cache URL-backed initialisé par un snapshot local canonique frais du même publisher**, API additive typée. Pas de warmup de test, pas de retour au cache inline, pas de waiters singleflight. Cette initialisation implémente ADR0304 §17 et ADR0308 §6 : le snapshot autoritatif peut exister au boot, mais sa confiance expire à 60s. Le publisher/registry et les politiques de trust/acquisition ne changent pas. **Aucune ADR nouvelle ni amendement de cible Accepted requis pour cette extension compatible et bornée** ; une autre autorité de seed, une confiance persistante, une attente sur miss ou une modification de TTL serait une décision séparée à ratifier. Ce contrat n'est ni un Accept autonome ni une preuve de livraison.

Sources vérifiées : `.tmp/ci06-sdk-jwks-proposal.md` ; `rustycog-http/src/jwt_handler.rs::{from_parts,from_config_with_inline_jwks,verify_rs256}` ; `jwks.rs::{from_inline_json,from_url,parse_jwks_document,get_cached_at,apply_snapshot,run_periodic_refresh}`. Le branchement inline ignore la URL ; le candidat dirty URL-only laisse le cache froid. Le brief `.cursor/review-briefings/20261004T0933Z-rust-perf-sdk-jwks-transport-ba69c9e.md:25–36` conserve l'acquisition nonwaiting et le token async cancellation-safe ; son BLOCK historique n'est pas le verdict courant.

### API et validation choisies

Nom proposé : `UserIdExtractor::from_config_with_seeded_jwks(AuthConfig, LocalJwksSeed) -> Result<Self, CommandError>`. **Pas une entrée JSON brute retimestampée à la construction.** `LocalJwksSeed` est opaque, move-only, non Clone/non Serialize/non Deserialize ; champs privés : snapshot parsé, URL autoritative normalisée et Instant d'acquisition. Le factory async `LocalJwksSeed::capture(authority_url, read_local_snapshot)` capture l'Instant **avant** l'await du lecteur local, parse/borne le résultat et retourne le seed. Aucun Instant fourni par le caller, aucune relecture des seuls fichiers PEM. Cette factory ne fait elle-même aucun HTTP ; le callback IAM ne fait qu'une lecture primaire via le publisher métier.

La provenance est une obligation du composition root de confiance : **JSON valide ne prouve pas l'autorité**. Cette API n'accepte jamais un body/header JWT, une configuration externe persistée ou une copie ancienne comme seed. Le SDK ne peut attester une DB IAM par la seule syntaxe ; ne pas lui attribuer cette garantie. URL du seed et `AuthConfig.jwt.jwks_url` doivent désigner exactement le même endpoint normalisé (scheme/host/port/path/query ; userinfo/fragment refusés, HTTP(S) seulement). URL interne de transport et issuer public peuvent différer selon le câblage existant : pas de règle naïve host(iss)=host(URL).

Constructor : URL obligatoire, RS256 effectivement autorisé, issuer plateforme et audience non vides, seed âgé strictement <60s ; sinon erreur de boot, **aucun fallback inline**. Taille <= limite live 1MiB, mêmes parser/contrôles canonisés RSA/kid/duplicate/iss/status/trust_scope/organization_id que le fetch live ; issuer des clés plateforme = issuer configuré. Les org keys gardent leur issuer/owner, sans réécriture plateforme. Audience vient de config, jamais du seed. L'API reste compatible avec la fenêtre HS256 explicitement configurée et son secret indépendant, sans l'activer ni dériver de HMAC depuis RSA. `with_mesh` reste appliqué.

Construire l'état URL-backed comme `from_url` (client TLS normal, redirects none, timeout/connect500ms, throttle250ms, negative-cache borné, poll existant). Y installer le snapshot seed avec **son ancien Instant**, pas `Instant::now()` à l'installation ; tous chemins de lookup/final `still_authorizes` utilisent cette fraîcheur. Ne pas mémoriser une seconde copie bootstrap servant de fallback. Refresh réussi = remplacement intégral, y compris vide/Pending/Revoked ; expiration, erreur, hit, cancellation ou negative hit ne renouvellent rien. Les constructeurs inline et URL existants restent sémantiquement inchangés. Le poll commence paresseusement comme aujourd'hui, sans HTTP à la construction ; known seeded kid n'acquiert pas le token de refresh.

### Source IAM et modules

`services/IAMRusty/setup/src/app.rs::setup_jwt` construit aujourd'hui un JWKS mono-clé depuis la ligne bootstrappée (~978–1003). **Ce `_inline_jwks` n'est pas un snapshot canonique complet et n'est pas accepté comme seed.** Après bootstrap et `setup_iam_usecases`, avant le move dans `CommandRegistryFactory::create_iam_registry`, capturer le seed via `usecases.token.get_jwks()` puis le même serializer compact que le publisher. `application/src/usecase/token.rs::TokenUseCaseImpl::get_jwks` lit `jwks_publication_snapshot`, filtre à `snapshot.as_of` avec access TTL et valide le DTO complet ; vide est succès, erreur DB/clé corrompue est erreur, jamais bootstrap. Le même writer/TTL/endpoint sert à la future route JWKS de cette instance. Remplacer seulement le candidat URL-only au wiring de l'extracteur (~452). Local et remote SigningProvider passent le même publisher : pas de chemin spécial PEM.

SDK concernés (dans sibling de publication seulement) : `rustycog-http/src/{jwks.rs,jwt_handler.rs,lib.rs,jwt_rs256_tests.rs}` + tests de cache existants ; export étroit du type, module jwks privé. Aucun schema/config HTTP/port/migration/mesh/ext-authz à changer. Les corrections IAM dirty restent préservées ; contrôler le delta avant remplacement.

### Gates obligatoires

- Units/cache déterministes : capture avant read/parse et construction différée, refus seed>=60s/URL différente/config manquante/document invalide ou trop gros ; pas de ré-age par consommation ; fresh/empty seed ; Pending/Revoked jamais trusted ; issuer/aud/org/alg/kid/signature/typ incohérents refusés.
- Constructor/factory : zéro HTTP avant écoute ; poll paresseux conservé. Premier burst concurrent sur le même platform kid frais : tous les tokens valides réussissent, aucun warmup ni acquisition réseau nécessaire.
- URL live : nouveau org kid résolu puis route Account IAM renvoie **403**, pas 401 ; remplacement de matériau/métadonnées/empty retire l'ancien seed, recheck final concurrent refuse une clé retirée ; pas d'union/bootstrap. Outage + JWT non expiré après60s ->401 ; seul un nouveau snapshot autoritatif validé peut rétablir la confiance.
- Acquisition inchangée : unknown/stale burst borné et fail-fast, negative TTL/cardinalité/throttle250ms conservés, cancellation libère l'unique token sans rajeunir snapshot ; tests redirects/TLS/timeout/content-size déjà existants restent exécutés.
- IT ciblées, après gates SDK et intégration publiée : `user::test_get_user_concurrent_requests_with_same_token`, `internal_provider_token::test_internal_provider_token_concurrent_requests_same_user`, `organization_account_guard`, et chemin réel publisher/TTL de `signing_admission`. Assertions actuelles intactes ; autres fails/collisions ne constituent pas une preuve de cette seam. Le témoin OAuth cleanup reste hors de cet addendum.
- Reviews correctness + test + security + rust-perf sur nouveau delta. Gates SDK `--locked --all-targets` de sa matrice CI et unités pures -> push `main` autorisé -> checkout détaché publié -> seul gitlink AIForAll ; puis build matrice IAM exact, gates ciblées et une validation intégrée finale parent. Aucun test/runtime lancé par l'architecte. Le candidat local demeure **NOT MERGE-READY** jusqu'à ces preuves.

**Inventaire de validation corrigé par le test-reviewer (2026-10-06)** : les quatre sélecteurs initiaux ont exécuté 43 tests, dont les 22 nouveaux. Quatre tests purs existants de métadonnées TLS, configuration mTLS et skew JWT restent sélectionnés séparément dans `.tmp/ci06-sdk-transport-gate-selection.md`. Aucun test pur existant n'a été trouvé pour le timeout réseau live de 500ms ou la borne du body HTTP ; ne pas présenter ces deux réglages inchangés comme couverts dynamiquement. Les tests du parser couvrent la taille du document, ce qui n'est pas une preuve de borne du transport HTTP. Cette correction d'inventaire ne change ni les politiques de confiance, ni les assertions, ni la durée d'expiration.

## Addendum 2026-10-06 — optimisation ciblée num-bigint-dig, recherche upstream

**Faits source uniquement.** Les trois lockfiles AIForAll, checkout consommé `AIForAll/rustycog`, sibling autonome `../rustycog` verrouillent **rsa0.9.10 → num-bigint-dig0.8.6**. Aucune section `[profile.*]` dans leurs trois manifests racines. `.cargo/config.toml` AIForAll existe mais ne définit ni profile ni rustflags ; aucun équivalent SDK inspecté. Le workflow CI n'a pas de CARGO_PROFILE/RUSTFLAGS/--profile/--release/opt-level explicite. Cela établit l'absence de configuration déposée, pas l'absence d'un override injecté au runtime ; vérifier les flags effectifs lors de la mesure future, sans lire de secrets.

Sources officielles exactes :
- https://raw.githubusercontent.com/RustCrypto/RSA/v0.9.10/README.md, lignes32–43 : génération `RsaPrivateKey::new` exceptionnellement lente sans optimisation ; recommande spécifiquement `[profile.dev.package.num-bigint-dig] opt-level=3` pour obtenir une grande partie des gains sans tout optimiser.
- https://docs.rs/rsa/0.9.10/rsa/ : même recommandation, dépendance num-bigint-dig ^0.8.6.
- https://doc.rust-lang.org/1.94.0/cargo/reference/profiles.html : `test` hérite de `dev`, profiles lus au seul root workspace, ceux des dépendances ignorés ; config/env prioritaires ; optimisation des génériques dépend aussi de la crate d'instanciation.

**Paquet minimal recommandé, avant cache RSA/transformations DTO si les gates fonctionnels sont prêts :** ajout de ces seules lignes au `Cargo.toml` racine AIForAll, par l'éditeur autorisé :

```toml
[profile.dev.package.num-bigint-dig]
opt-level = 3
```

Pas de `[profile.test]` dupliqué nécessaire à ce stade ; pas de profile global, RUSTFLAGS, release-switch, dépendance/version/feature, assertions, overflow/debug-assertions, paramètres RSA/probe ou hardware modifiés. Ne pas copier `[profile.debug]` également cité par le README : ce n'est pas le profile builtin Cargo à employer. L'override optimise cette package uniquement, **pas transitivement rsa ni toutes les crates** ; les génériques peuvent limiter le gain.

Le root consommateur est indispensable : une configuration ajoutée uniquement au SDK n'affecterait pas les builds IAM. Un ajout identique au manifest du sibling SDK ne servirait qu'à ses builds autonomes ; candidat séparé après mesure utile, hors des six fichiers SDK actuellement gelés et hors gate en cours. Aucun changement du submodule consommé ici. Une réutilisation large nécessite les roots consommateurs appropriés, pas un profile « exporté » par le SDK.

**Mesure future unique et bornée :** après fin des baux/gates actuels, un seul créneau Cargo en Docker, quota confirmé2CPU inchangé et cache Linux existant. Comparaison courte avant/après sur le même sélecteur IAM existant qui boot une fixture RSA2048, par exemple le test concurrent `user` déjà requis ; pas de full suite. Chronométrer séparément compilation/recompilation et phase RSA/boot/exécution ; changement de profile invalide certains artefacts et coûte du build. Génération aléatoire implique variance : publier la limite d'une comparaison courte, pas extrapoler à cinq minutes. Conserver llvm-cov/sélection/features et instrumentation identiques ; vérifier couverture/scénarios de la sélection principale lors du benchmark final prévu, car optimisation/inlining peuvent changer le mapping de couverture (pas les assertions). Une mesure hors couverture seule n'établit pas le budget sous llvm-cov.

**Priorité/coexistence :** ce candidat configuratoire peu coûteux conserve chaque génération et sa force ; le cache du matériau neutre élimine des générations mais impose l'audit d'isolation déjà décrit. Le profile ne supprime ni parsing redondant, ni accounting quadratique, ni fixtures retenues : ces autres candidats restent conditionnés aux timers. Aucun gain mesuré actuellement, aucune promesse5min. Aucune ADR nouvelle : réglage réversible de build sous contrat crypto inchangé.

**Signal documentaire hors audit :** ce README versionné mentionne explicitement Marvin/RUSTSEC-2023-0071 et un risque de récupération de clé. C'est un avertissement upstream préexistant, non une démonstration d'exploitabilité des chemins IAM inspectés ; aucun audit supply-chain, changement de dépendance ou remédiation automatique dans ce paquet. L'optimisation n'est pas une preuve de constant-time.
