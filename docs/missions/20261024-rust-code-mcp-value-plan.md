# Petite campagne de valeur rust-code-mcp — plan avant exécution

## Contrat
Demande utilisateur : juger l'utilité annoncée des outils, pas leur seule capacité
à répondre. Exécution directe, sans agents supplémentaires. Corpus réel AIForAll,
profil **local-qwen3-8b** explicite pour tout outil sémantique. Aucun correctif,
refactor ou décision d'architecture appliqué pendant cette campagne.

Préflight : HEAD `93e376c5530a439aaafe61a64d3e881865c1acf1`, 989 entrées du
manifeste inchangées, backend v3 PID 41512, caches vides, background sync arrêté.
GPU global : 3348 MiB utilisés / 20975 libres. Index/graphe déjà disponibles.
Aucun Cargo, Docker, Kind ou WSL nécessaire ; pas de rebuild/reindex/restart.

## Mesure et verdict
17 appels métier dans 8 scénarios, séquentiels, réponses brutes et durées archivées.
Budget indicatif < 5 minutes, délai maximal par appel 120 s ; un dépassement est
une limite observée, pas une permission de lancer des reconstructions.
PASS = réponse exploitable qui satisfait l'oracle source ; PARTIAL = valeur réelle
mais couverture/bruit/présentation décevante ; FAIL = promesse discriminante manquée ;
INCONCLUSIVE = preuve insuffisante. Un HTTP 200 ou une liste non vide ne vaut pas PASS.
La latence froid/chaud est observée, aucun ratio arbitraire imposé.

## Scénarios et oracles préalables

### V1 — Retrouver un comportement sans connaître le symbole (3 appels)
Question : « a service should go unhealthy when its configured message backend
silently becomes an inert adapter instead of a live transport ».
`search` froid puis même appel chaud, et `get_similar_code` dense seul, top 5.
Utile si au moins un résultat top 5 mène au code qui classe/signale le fallback
ou le rend non prêt, pas seulement à des mots proches. Attendus admissibles :
`signal_queue_status`, `classify_noop`, `classify_transport`, `is_degraded`,
`queue_check` dans `crates/readiness/src/`. Comparer pertinence dense/hybride,
bruit des tests et coût froid/chaud. Sources : classify.rs, probe.rs, factory.rs.

### V2 — Trouver un vrai voisin de refactor (1 appel)
`similar_to_item(telegraph_infra::event::consumer::EventConsumer::transport_status)`.
Top 5, threshold 0, kind Function. Positif connu indépendant :
`services/Lazaret/infra/src/event/consumer.rs:46-50`, méthode
`KvPurgeEventConsumer::transport_status`, même corps et rustdoc que Telegraph
`consumer.rs:56-60` (retrouvés via Serena avant le test).
Valeur : voisin exact utile dans top 5, seed lui-même exclu. Ce voisin est une
piste de duplication, pas une preuve qu'une abstraction commune est souhaitable.

### V3 — Préparer une modification transversale sans oublier de service (3 appels)
`who_calls(create_signaled_event_consumer)`, `callers_in_crate(telegraph_infra)`
puis `calls_from(create_signaled_event_consumer)`.
Oracle : appels réels à la factory dans Telegraph consumer.rs:40, Manifesto
consumer.rs:56 et Lazaret consumer.rs:37 (grep exact préalable) ; filtre Telegraph
sans les autres consommateurs ; sorties vers classify_consumer et signal_queue_status
conformes à factory.rs:78-79. Vérifier les spans sur les octets sources et séparer
les véritables invocations des références de types que `calls_from` inclut aussi.

### V4 — Préparer un changement d'API avec signatures et pagination (3 appels)
`function_signature(create_signaled_event_consumer)`, deux pages limit=1 de
`functions_with_filter(readiness, is_async=true, has_param_type=QueueConfig)`.
Oracle factory.rs:42-47,73-76 : exactement les deux factories async, paramètre
`&QueueConfig`, retour consumer `Result<SignaledConsumer, ServiceError>`. Les deux
pages doivent être distinctes et annoncer le total=2 : aucun résultat caché pris
pour une absence. Une page seule ne valide pas la couverture.

### V5 — Comprendre l'API publique et les imports à préserver (2 appels)
`get_declared_reexports(readiness)`, `re_export_chain(classify_consumer)`.
Oracle `crates/readiness/src/lib.rs:14-23` : réexports explicites des fonctions,
types et HTTP/probe ; classify_consumer est déclaré sous classify et réexporté
à la racine. Valeur : chemin public de migration visible et déclaration canonique
distincte, pas confusion entre un import interne et un réexport public.

### V6 — Transformer une relation réelle en garde-fou architectural (1 appel)
`forbidden_dependency_check` avec 3 règles **synthétiques**, pas des ADR réelles :
positive readiness→rustycog_framework, même règle avec exception readiness,
négative readiness→telegraph_infra. Oracle manifeste readiness/Cargo.toml:15 et
factory.rs:9-12 ; aucun lien inverse vers Telegraph. Attendu : une violation de
la seule règle positive, portant des symboles/ref counts crédibles.
Ne pas présenter cette interdiction artificielle comme un défaut du projet.

### V7 — Obtenir un audit de surface avec contrôles positifs et négatifs (2 appels)
`derive_audit(readiness, required_derives=[Debug])`, puis attributs de ComponentStatus.
Oracle : SignaledPublisher et SignaledConsumer (factory.rs:19-35) n'ont aucun
derive Debug ; ComponentStatus est déjà `#[derive(Debug, Clone, PartialEq, Eq)]`
dans classify.rs et ne doit pas être signalé. Vérifier cible/file/span et ne pas
transformer « manque un derive » en patch automatique : les champs doivent aussi
permettre Debug. Valeur : findings navigables, pas seulement un compteur.

### V8 — Passer d'une question à une carte utile du changement (2 appels)
`build_codemap(task_prompt=question V1, embedding_profile=8B, no_rerank, depth=2,
max_nodes=20)` puis même budget avec seed explicite de la factory consumer.
Oracle : carte comportant du code readiness pertinent, ses liens réels et
fichiers/spans ; carte explicite reliant factory/classify/signal. Évaluer la
sélection de seeds sémantiques, les diagnostics d'échec de raccordement et le
bruit. Répondre avec des nœuds n'est pas suffisant : la carte doit aider à savoir
où intervenir et qui est affecté. Le cap de 20 n'est pas une preuve d'exhaustivité.

## Hors campagne et protection
Pas d'essai exhaustif des 55 outils. Pas de `semantic_overlaps` global (max_pairs
limite la sortie, pas le travail), d'audits AST rechargeant tout rust-analyzer,
de skeleton généré ni de commandes de suppression. Aucune source sensible brute
ne doit être copiée dans le rapport public. Index/LMDB existants préservés.
Seuls les caches runtime créés pour le workspace sont libérés en fin, même sur
échec. Invariants fichiers et état runtime contrôlés après ; aucun modèle épinglé.

Preuves privées sous `%LOCALAPPDATA%/rust-code-mcp/` :
`value-campaign-preflight.json`, `value-campaign-20261024.json`, journal natif
et preuve finale. Verdicts source/analyse par le même agent, pas review indépendante.
