## Handoff et continuité (décision 2026-10-07)

Le harness est proportionné (`.cursor/rules/00-agent-harness.mdc`). Sous OpenCode, la session racine exécute directement ou pilote Ensemble, sans orchestrateur intermédiaire. Les autres clients conservent leur routage historique. Contrats d'architecture : `docs/contracts/` (canon secondaire, écrits par `architecte`, référencés depuis les ADR).

- Chaque agent lit le ledger de handoff du lot : `.cursor/handoffs/` (règle `.cursor/rules/agent-handoff.mdc`). Sous Ensemble, les membres rendent leurs deltas et le lead les persiste sans écritures concurrentes. Rien d'essentiel ne reste chat-only.
- Suite de travail sur un même lot = reprendre la session subagent existante (sessionID), pas un spawn neuf.

<!-- ensemble-project-setup:policy:v1 -->
## Coordination OpenCode / Ensemble (décision 2026-10-10)

- Charger `opencode-ensemble` avant de coordonner une équipe. Travailler directement lorsqu'un seul agent suffit ; annoncer direct ou Ensemble. La racine devient le lead, sans coordinateur intermédiaire.
- Le lead choisit profils, taille et dépendances selon les lots réellement indépendants. Aucun pipeline obligatoire. Chaque lot a un propriétaire, un périmètre, des invariants, un livrable et des critères de validation. Les décisions structurelles restent à `architecte`.
- Découvrir une fois par périmètre : réutiliser sources et ledgers valides ; justifier toute nouvelle exploration. Lire la référence de continuité du skill avant héritage/handoff. Continuer avec le même membre ; handoff pour changer de spécialité dans la même session, fork pour une branche indépendante, checkpoint pour une frontière exclusive réelle, compact_fork depuis une source distincte et inactive. Ne pas compacter à nouveau pour chaque enfant.
- Forks limités aux sources de l'équipe ; pas de synchronisation live des contextes. Le lead transmet les changements de contrat aux membres concernés et tient le ledger canonique (chemin absolu, base Git, faits/hypothèses distincts, décisions, tentatives, sources/sessionID). Entre équipes, réutiliser le ledger, sans promettre un héritage natif arbitraire.
- Reviewers indépendants : contexte fresh, diff, contrats et faits vérifiés, sans conclusions du développeur. Reviews proportionnées à la surface du diff ; briefings persistés avant correction.
- Seul le lead crée des membres. Un handoff garde rôle, tâches, répertoire et modèle sélectionné : il ne donne ni leadership ni isolation et n'applique pas automatiquement le modèle du profil cible. Respecter les modèles configurés ; expliciter une éventuelle sélection séparée. Les membres restent des feuilles, même avec mode all.
- Worktrees isolés pour les écrivains indépendants ; héritage avec changement de répertoire uniquement sur arbres propres et HEAD compatibles. Préserver les changements utilisateur : aucun commit/stash/reset automatique ni repli silencieux vers fresh ou un répertoire partagé pour contourner une erreur. L'index GrepAI canonique n'inclut pas les modifications non mergées des worktrees temporaires : utiliser leurs fichiers/symboles pour celles-ci.
- Le lead intègre et vérifie les résultats réels, coordonne l'unique slot Cargo Windows et les baux infra, et contrôle le cleanup avant de rendre. Pas de polling d'attente ni de validation redondante. L'héritage économise la redécouverte, pas nécessairement les tokens d'entrée.
- Après deux tentatives techniques substantielles infructueuses sans progrès vérifiable sur le même blocage, transférer au `emergency-thinker` ; une troisième n'est permise que pour une hypothèse différente, étayée et bornée (maximum trois). Conserver le registre même après changement de membre. Attentes/autorisations/infra indisponible ne sont pas des tentatives. En équipe, le handoff du lead conserve son leadership ; un membre demande ce transfert au lead, sans s'attribuer la coordination. Le modèle Astra doit être sélectionné séparément si nécessaire.
<!-- /ensemble-project-setup:policy -->

## Compilation et tests locaux (décision utilisateur 2026-10-07)

L’hôte est Windows : **la compilation et les tests développement tournent nativement sous Windows**, pas dans Docker. Docker ne sert qu’à deux choses : (a) construire les images Linux destinées à Kind (`docker compose build`, `build-artifacts`, `kind load` — Kind ne compile pas), et (b) l’infrastructure tierce démarrée automatiquement par les fixtures testcontainers des tests d’intégration (Postgres, LocalStack, OpenFGA, etc.).

- `cargo build`, `cargo test` et `cargo check` se lancent sur l’hôte Windows, par défaut. NE PAS compiler ou tester via Docker pour le développement courant, les tests unitaires ou les tests d’intégration : c’est interdit sans demande explicite de l’utilisateur. Un seul `cargo` à la fois.
- Le dossier `target\` vit sur le disque E: (NVMe), configuré via `.cargo/config.toml` (`target-dir = E:\cargo-target\AIForAll`). Ne jamais le monter ou le partager avec un conteneur Linux : un `target` Windows (`x86_64-pc-windows-msvc`) est incompatible avec les artefacts Docker (`x86_64-unknown-linux-gnu`).
- Les tests d’intégration utilisent le harness testcontainers existant, qui démarre lui-même ses conteneurs tiers via le daemon Docker (bridge testcontainers). Pas de build d’image ni de run de service applicatif dans Docker pour ça : le binaire testé est le `.exe` Windows de `target\`.
- Les images Linux pour Kind continuent d’être compilées dans Docker (BuildKit). C’est du build de livraison, pas un environnement de dev.

## Cycle de vie du runtime local

- Campagnes de preuves Apparatus/IT/E2E : charger aussi `.agents/skills/aiforall-test-evidence/SKILL.md` avant le préflight. Inspecter les suppressions/remplacements transitifs du harness, pas seulement sa fonction `ensure`. Le cycle local m6 Pod/AdmissionRecord est accepté (2026-10-18), mais noms/digests ne prouvent jamais la propriété ; documenter les UID remplacés, ne pas annoncer une préservation globale ni autoriser implicitement un environnement distant/production.

- Charger `.agents/skills/local-runtime-lifecycle/SKILL.md` avant démarrage ou fin de Docker/Kind/WSL, E2E, test local ou IT ; canon court : `.cursor/rules/local-runtime-lifecycle.mdc` (OpenCode n'applique pas automatiquement les MDC).
- Runtime à la demande seulement : E2E/test manuel local → `kind-aiforall-local` (défaut) ou, uniquement sur autorisation user explicite (2026-10-04) pour un scénario full profile multi-nœuds, `kind-aiforall-local-full` ; IT → fixtures testcontainers du harness existant, jamais Kind ni migration Compose. Ne pas réveiller un runtime arrêté pour du travail documentaire/statique ; aucun démarrage de Kind pour l'E2E avant la fin des corrections et de la phase IT finale (les builds Windows / pure unit gates en cours d'implémentation restent autorisés — un slot cargo, ledger propre).
- Le parent tient l'inventaire AVANT et le bail partagé des IDs créés/redémarrés explicitement pour la tâche ; ni nom ni image ne prouvent la propriété. Les workers rendent cet inventaire sans arrêter un runtime encore requis par le parent.
- **Fin de campagne IT — stop possédé obligatoire** : une campagne IT testcontainers (`it-native`, `cargo test -p <pkg> --tests`) laisse zéro container possédé running à sa fin, y compris sur échec — stop gracieux par IDs tracés (fin de batch et sur échec), retrait des `Exited(0)` possédés, rapport d'inventaire avant/après. Piège : `docker ps` tronque les IDs par défaut → recueillir/recouper les IDs avec `--no-trunc` ou depuis les ledgers `fixture-*.jsonl` ; jamais par nom, jamais prune. Canon : `.cursor/rules/docker-resource-registry.mdc`. **Intégré et prouvé** (mission `docs/missions/20261019-it-fixtures-lifecycle.md` : campagne Manifesto+Hive — 74 créés/74 retirés, pic borné à 4, 0 flake).
- Avant réponse finale, succès ou échec : arrêter gracieusement les ressources possédées devenues inutiles (permission permanente pour ce stop réversible, annoncer périmètre/perte de mémoire volatile), vérifier les fuites ; aucune suppression/prune/reset. Rétention seulement pour travail réellement actif/bail parent ou demande explicite de keepalive.
- Pour libérer la RAM, arrêter Docker Desktop puis `wsl --shutdown` seulement sans autre workload/bail, propriété inconnue ni cluster protégé affecté ; sinon signaler la RAM résiduelle et demander un arbitrage ciblé. Vérifier états + vmmem/RAM disponible sans Docker CLI après shutdown.
- Persister un petit ledger non secret avant les opérations longues ; signaler tout nettoyage bloqué. Politique de prompts, pas de moniteur d'inactivité installé : déconnexion brutale/kill ne garantit pas le teardown. Ne pas interrompre git ni un publisher hôte en arrière-plan.

## Recherche de code (GrepAI)

Canon court : `.cursor/rules/grepai-first.mdc`. L'index vit dans `.grepai/` (watcher : `grepai watch --status`).

- Exploration sémantique (« où/comment fonctionne X », concepts, comportements) → MCP GrepAI d'abord (`grepai_search` ; graphe d'appels : `grepai_trace_callers`/`grepai_trace_callees`).
- grep/Glob réservés au matching exact (renames, chaînes d'erreur, noms de symboles connus).
- Résultat vide → vérifier `grepai status` avant d'abandonner ; index vide/watcher arrêté = signaler la panne, ne pas la masquer par grep.

## Problèmes infrastructure

Routage des diagnostics locaux (Docker Desktop, Compose, Kind `aiforall-local` et `aiforall-local-full` — liste close, Envoy du dépôt). Pas de prod. Jamais `kind-apparatus-p4-it` ni `rancher-desktop` ni tout autre contexte non listé.

- **Docker / Compose** → `container-runtime-debugger` (skills `.agents/skills/docker-*`) — **uniquement sur autorisation user explicite** (décision 2026-10-07 : Docker n'est plus l'environnement de build/test de dev ; cet agent ne sert plus que les problèmes du runtime lui-même).
- **Kubernetes / Kind** → `k8s-operator` (read-only par défaut).
- **Envoy / routing / TLS / networking** → `envoy-network-debugger` (skill `envoy-runtime-debug`).
- **Diagnostic kernel / TCP / DNS** → Inspektor Gadget MCP **si** déployé in-cluster et Ready ; sinon INCONCLUSIVE.
- **Après un changement infra significatif** → `infra-verifier` (PASS/FAIL/INCONCLUSIVE, ne modifie rien).

Parallèle autorisé. Exemple HTTP 503 : `envoy-network-debugger` + `k8s-operator`, puis `infra-verifier`.

Règles toujours appliquées : `.cursor/rules/infra-safety.mdc`. Détail d'installation : `docs/cursor-infra-agents.md`.
