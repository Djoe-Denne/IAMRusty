# Prompt — Implémentation Apparatus P0

Tu travailles dans le dépôt `C:\Users\djden\source\repos\AIForAll`.

## Mission

Implémente **P0 — contrats Apparatus et Apparatus KV de référence**, conformément aux ADR ratifiées. Livre le code, les tests et les mises à jour documentaires factuelles. Ne dépasse pas P0.

Conduis la tâche de bout en bout : exploration, choix d’emplacement minimal, implémentation, tests ciblés, formatage, diagnostics et compte rendu. Ne demande une clarification que si une décision matérielle absente des ADR bloque réellement l’implémentation.

## Sources de vérité à lire avant de coder

1. `AGENTS.md` et les règles du workspace.
2. `docs/adr/README.md`.
3. ADR-0001 à ADR-0005 dans `docs/adr/`.
4. Avec QMD CLI, collection `aiforall-wiki` :
   - `projects/manifesto/concepts/apparatus-platform.md`
   - `projects/manifesto/concepts/apparatus-bindings-and-lifecycle.md`
   - `projects/manifesto/concepts/apparatus-capabilities-and-isolation.md`
   - `projects/manifesto/references/apparatus-source-reconciliation.md`
   - `projects/manifesto/references/apparatus-implementation-plan.md`
   - `projects/manifesto/references/apparatus-factory-and-distribution.md`
   - `projects/manifesto/references/apparatus-ui-and-protocol.md`
5. Le workspace `Cargo.toml`, la structure multi-crates de `Manifesto/` et les conventions de tests existantes.

Si tu touches une API RustyCog, lis `.agents/skills/rustycog/SKILL.md` et uniquement ses références pertinentes. P0 ne nécessite normalement ni nouveau service ni composition root RustyCog.

`Accepted` décrit une **cible ratifiée**. Les champs `Réalité` décrivent séparément le code présent. Ne prétends jamais que Factory, gateway ou runtime existent.

## Périmètre P0 obligatoire

### 1. Contrats canoniques

Créer l’emplacement Rust minimal et réutilisable pour :

- le schéma versionné de `apparatus.toml` ;
- les identités Apparatus, release et binding ;
- les DTO release, binding, configuration et opération ;
- le protocole backend privé : découverte, health/readiness, bind, configure, invoke et unbind ;
- les erreurs de validation versionnées et stables ;
- les modes UI déclaratifs `absent`, `schema` et `sandbox`, sans implémenter le host ;
- la taxonomie minimale de capacités nécessaire à la référence KV.

Favoriser un contrat Rust pur, sans Axum, SeaORM, client OpenFGA, JWT, AWS ou dépendance au runtime Manifesto. Le nom et l’emplacement précis des crates sont des choix d’implémentation : inspecte le workspace avant de les fixer.

### 2. Validation et identité immuable

Implémenter un parseur/validateur déterministe :

- une seule syntaxe de manifeste : TOML ;
- version de schéma distincte de la version SDK et du protocole wire ;
- SemVer déclarative distincte de l’identité d’installation ;
- digest déterministe du descripteur canonique, avec algorithme et canonicalisation explicitement documentés et testés ;
- rejet de `latest`, d’une branche ou d’un tag flottant comme identité d’installation ;
- rejet des versions majeures inconnues, capacités inconnues, champs de bypass `trusted_*` et credentials dans les contrats ;
- limites de taille et d’identifiant explicites, cohérentes avec `component_type` (100 caractères maximum).

L’admission de ce digest n’appartient pas à P0. Le parser, le harness ou un outil de développement ne produisent jamais `VALID` ou `VERIFIED`.

### 3. Apparatus KV de référence

Implémenter un Apparatus de référence minimal :

- backend Rust ;
- stockage abstrait `kv-v1`, fourni par le harness en mémoire ;
- UI `schema`, sans iframe ni bundle Vite ;
- aucune capacité réseau ;
- aucune lecture de variable d’environnement sensible ;
- opérations bind/configure/invoke/unbind idempotentes lorsque le contrat l’exige ;
- isolation des clés par `binding_id` imposée par l’adaptateur, jamais par un préfixe choisi par le plugin.

Son `apparatus_id` concret est un choix d’implémentation, pas une nouvelle décision d’architecture. Choisis un nom non ambigu et localise ce choix dans le code de référence, pas dans le contrat générique.

### 4. Harness P0

Créer un harness **in-process uniquement pour les tests** :

- aucun bearer IAM, JWT interne ou secret HMAC ;
- aucun accès SQL, réseau interne, SQS/Kafka ou filesystem privilégié ;
- aucune prétention d’isolation de production ;
- aucun statut `VALID` ou `VERIFIED` ;
- API suffisamment fidèle pour qualifier les contrats wire et rejouer bind/configure/invoke/unbind.

Le harness n’est pas un runtime Apparatus et ne doit pas être chargé dans Manifesto ou `oodhive-monolith` comme mécanisme de production.

## Tests d’acceptation P0

Ajouter au minimum des tests couvrant :

- manifeste valide et sérialisation stable ;
- digest identique pour une entrée canonique identique ;
- modification sémantique entraînant un digest différent ;
- rejet de `latest`, refs flottantes, version de schéma inconnue et capability inconnue ;
- rejet de champs `trusted_skip_gateway` ou équivalents ;
- absence de credentials/secrets dans les DTO sérialisés et erreurs ;
- limites d’identifiant et de payload ;
- bind/configure/unbind idempotents ;
- deux bindings ne lisent jamais le même namespace KV ;
- référence KV sans capacité réseau ;
- `sandbox` accepté comme déclaration seulement, sans créer de host ou conclure à une isolation.

Les tests doivent être déterministes et fonctionner sans Docker ni service externe.

## Hors périmètre strict

Ne pas ajouter :

- migration SQL, table catalogue, backfill ou route Manifesto ;
- contrôleur, worker, polling, desired/observed state, lease ou fencing ;
- Factory, CLI, proc-macro, Git checkout, OCI, registry, signature ou admission ;
- gateway réseau, mTLS, identité workload réelle, secret manager ou proxy egress ;
- Kubernetes, Docker runtime, WASM/WASI ou nouveau microservice ;
- host UI, iframe, CSP, MessageChannel ou serveur de bundles ;
- modification OpenFGA, sentinel-sync ou événements managed ;
- statut `VALID`/`VERIFIED` persistant.

Si un besoin P1+ apparaît, documente-le comme suivi sans l’implémenter.

## Contraintes de qualité

- Respecter les lints du workspace (`unsafe_code = forbid`, Clippy pedantic/nursery/cargo).
- Éviter `unwrap`/`expect` dans le code de production.
- Documenter les types et erreurs publics ; ajouter `# Errors` lorsque requis.
- Garder les dépendances minimales et réutiliser les versions du workspace.
- Préserver les modifications utilisateur et ne pas lancer de commande Git destructive.

## Vérification

Exécuter au minimum :

1. `cargo fmt --all -- --check`
2. `cargo check` ciblé sur les crates ajoutées/modifiées
3. `cargo test` ciblé sur P0
4. `cargo clippy` ciblé avec les niveaux du workspace, si le temps d’exécution reste raisonnable
5. diagnostics IDE sur les fichiers modifiés

Corriger toute régression introduite. N’élargis pas la tâche à des défauts préexistants sans rapport.

## Documentation de sortie

Mettre à jour uniquement les faits :

- champ `Réalité` des ADR concernées (`Partial` ou `Implemented` selon les preuves réellement livrées), sans modifier leur statut `Accepted` ;
- preuve P0 dans `apparatus-implementation-plan.md` ;
- documentation publique des nouveaux contrats et commande de tests.

Ne réécris pas les décisions ratifiées pour les adapter accidentellement à l’implémentation.

## Compte rendu final

Fournir :

- architecture et emplacement retenus ;
- fichiers/crates créés ou modifiés ;
- critères P0 satisfaits ;
- tests et commandes exécutés avec résultats ;
- éléments volontairement différés à P1–P6 ;
- éventuels risques ou questions réellement bloquantes.
