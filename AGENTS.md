## Handoff et continuité (décision 2026-10-07)

Le harness est maintenant proportionné (`.cursor/rules/00-agent-harness.mdc`) : tâches simples/moyennes exécutées directement par la session racine ; `orchestrator` requis pour le multi-lots/parallèle/heavy. Contrats d'architecture : `docs/contracts/` (canon secondaire, écrits par `architecte`, référencés depuis les ADR).

- Chaque agent écrit et lit le ledger de handoff du lot : `.cursor/handoffs/` (règle `.cursor/rules/agent-handoff.mdc`). Rien d'essentiel ne reste chat-only.
- Suite de travail sur un même lot = reprendre la session subagent existante (sessionID), pas un spawn neuf.

## Compilation et tests locaux (décision utilisateur 2026-10-07)

L’hôte est Windows : **la compilation et les tests développement tournent nativement sous Windows**, pas dans Docker. Docker ne sert qu’à deux choses : (a) construire les images Linux destinées à Kind (`docker compose build`, `build-artifacts`, `kind load` — Kind ne compile pas), et (b) l’infrastructure tierce démarrée automatiquement par les fixtures testcontainers des tests d’intégration (Postgres, LocalStack, OpenFGA, etc.).

- `cargo build`, `cargo test` et `cargo check` se lancent sur l’hôte Windows, par défaut. NE PAS compiler ou tester via Docker pour le développement courant, les tests unitaires ou les tests d’intégration : c’est interdit sans demande explicite de l’utilisateur. Un seul `cargo` à la fois.
- Le dossier `target\` vit sur le disque E: (NVMe), configuré via `.cargo/config.toml` (`target-dir = E:\cargo-target\AIForAll`). Ne jamais le monter ou le partager avec un conteneur Linux : un `target` Windows (`x86_64-pc-windows-msvc`) est incompatible avec les artefacts Docker (`x86_64-unknown-linux-gnu`).
- Les tests d’intégration utilisent le harness testcontainers existant, qui démarre lui-même ses conteneurs tiers via le daemon Docker (bridge testcontainers). Pas de build d’image ni de run de service applicatif dans Docker pour ça : le binaire testé est le `.exe` Windows de `target\`.
- Les images Linux pour Kind continuent d’être compilées dans Docker (BuildKit). C’est du build de livraison, pas un environnement de dev.

## Cycle de vie du runtime local

- Charger `.agents/skills/local-runtime-lifecycle/SKILL.md` avant démarrage ou fin de Docker/Kind/WSL, E2E, test local ou IT ; canon court : `.cursor/rules/local-runtime-lifecycle.mdc` (OpenCode n'applique pas automatiquement les MDC).
- Runtime à la demande seulement : E2E/test manuel local → `kind-aiforall-local` (défaut) ou, uniquement sur autorisation user explicite (2026-10-04) pour un scénario full profile multi-nœuds, `kind-aiforall-local-full` ; IT → fixtures testcontainers du harness existant, jamais Kind ni migration Compose. Ne pas réveiller un runtime arrêté pour du travail documentaire/statique ; aucun démarrage de Kind pour l'E2E avant la fin des corrections et de la phase IT finale (les builds Windows / pure unit gates en cours d'implémentation restent autorisés — un slot cargo, ledger propre).
- Le parent tient l'inventaire AVANT et le bail partagé des IDs créés/redémarrés explicitement pour la tâche ; ni nom ni image ne prouvent la propriété. Les workers rendent cet inventaire sans arrêter un runtime encore requis par le parent.
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
