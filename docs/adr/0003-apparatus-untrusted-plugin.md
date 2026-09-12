# ADR-0003 : Le code d’un Apparatus n’est pas digne de confiance et s’exécute hors des processus privilégiés

- Statut : Accepted
- Réalité : Partial
- Date : 2026-09-10
- Décideurs : Architecture AIForAll — ratification orchestrée du 2026-09-10
- Jalon concerné : P0 (modèle de confiance), P3–P4 (runtime réel)
- SuperSède : aucune
- SuperSédée par : —

## Contexte

Un Apparatus V1 est du Rust (et éventuellement du JS statique) fourni par un publisher. Cargo exécute `build.rs` et des proc-macros ; Vite exécute la config auteur. Ce n’est pas un build « safe » parce que le builder est connu.

Le monolithe `oodhive-monolith` compose déjà IAM, Hive, Manifesto, Telegraph. Y charger du Rust communautaire donnerait au plugin les secrets et le réseau de la plateforme.

`docs/project/Archi.md` visait des microservices composants avec registre Redis et découverte Kubernetes. Le dépôt n’a ni Factory, ni isolation de workloads Apparatus, ni ce registre.

## Décision

1. Le code auteur s’exécute dans un **processus OS et une identité de workload distincts** de Manifesto, du monolithe et des composants privilégiés : workers Factory, admission/signature, contrôleur runtime et capability gateway. Une séparation logique dans le même espace d’adressage ne satisfait pas cette exigence.
2. V1 : runtime **managed** seulement. Isolation **par projet et par binding**. Le manifeste peut *demander* d’autres modes ; l’opérateur décide ; `shared` / `organization` sont hors V1.
3. Le monolithe peut exposer les **API de contrôle** Manifesto. Il **ne charge pas** le binaire communautaire dans son processus. Un Apparatus est un workload distant dans les deux modes de runtime plateforme.
4. Un harness **in-process** (P0) est un double de test. Il ne constitue pas le modèle de confiance de production, ne reçoit aucun secret IAM et ne peut produire `VALID` ou `VERIFIED`.

## Conséquences

- P0 n’introduit pas de microservice Factory.
- Les tests d’isolation réseau / sandbox ne sont pas des mocks de NetworkPolicy (plan wiki).
- Refuser une installation si le profil d’isolation requis n’existe pas, plutôt que déployer « un peu moins isolé ».
- Officiel et communautaire : même confinement (ADR-0005).
- `Accepted` fixe ici la frontière de confiance. Il ne signifie pas qu’un moteur d’isolation ou un runtime Apparatus existe déjà.

## Alternatives rejetées

| Option | Pourquoi pas |
|---|---|
| Plugin in-process dans Manifesto / le monolithe | Partage secrets, mémoire, réseau interne |
| WASM/WASI en V1 | Autre toolchain et conformance ; direction conservée, pas le premier runtime |
| Isolation `shared` entre projets en V1 | Données, secrets et sessions à prouver d’abord |
| Commande de build libre dans le manifeste (`build = "…"`) | Exécution arbitraire côté Factory |

## Non décidé ici

- `APP-01` : budget, CPU, moteur d’isolation, registry, autorité de signature.
- Adaptateur de production, y compris le choix éventuel de Kubernetes.
- Forme exacte des workers Factory (P4).
- Scale-to-zero, OSB, Knative, etc.

## Références

- Wiki : `apparatus-platform`, `apparatus-capabilities-and-isolation`, `apparatus-factory-and-distribution`, `modular-monolith-runtime`
- Code : `monolith/src/runtime.rs`, Dockerfile Manifesto (image **du service**, pas une Factory Apparatus)
- Preuve d’implémentation (2026-09-10, P1 2026-09-12) : le harness `TestHarness` / `InMemoryKv` dans `apparatus-contracts/src/harness.rs` est TEST-ONLY et compilé uniquement avec la feature `test-harness` (absent des builds prod), namespacé par `binding_id`. Aucun code de plugin n’est chargé dans le monolithe ni Manifesto. Le harness ne produit ni statut `VALID`, ni `VERIFIED`. P1 : zéro workload tiers démarré (T2 5/5, T4 3/3 faux broker in-memory, 0 polling/worker, `Manifesto/infra/src/apparatus_outbox.rs`) ; gate T7 3/3 vert, 0 token P2 dans le prod Manifesto scanné par T7 (7 crates src) — l’absence de plugin in-process ne constitue pas un runtime isolé de production → `Partial` maintenu.
