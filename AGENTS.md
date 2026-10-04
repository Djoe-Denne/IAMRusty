## Compilation locale

L’hôte est Windows. Kind et les tests locaux tournent dans la VM Linux de Docker Desktop.

- Compiler dans Docker pour les images Kind et les tests locaux. Kind charge l’image (`kind load`), il ne compile pas.
- `cargo build`, `cargo test` et `cargo check` sur Windows uniquement si un binaire Windows est demandé, ou si l’outil ne peut pas tourner dans Docker. Un seul `cargo` à la fois.
- Ne pas partager `target\` entre Windows (`x86_64-pc-windows-msvc`) et Docker (`x86_64-unknown-linux-gnu`). Un `cargo check` hôte ne réutilise pas les artefacts Linux.

## Cycle de vie du runtime local

- Charger `.agents/skills/local-runtime-lifecycle/SKILL.md` avant démarrage ou fin de Docker/Kind/WSL, E2E, test local ou IT ; canon court : `.cursor/rules/local-runtime-lifecycle.mdc` (OpenCode n'applique pas automatiquement les MDC).
- Runtime à la demande seulement : E2E/test manuel local → `kind-aiforall-local` (défaut) ou, uniquement sur autorisation user explicite (2026-10-04) pour un scénario full profile multi-nœuds, `kind-aiforall-local-full` ; IT → fixtures testcontainers du harness existant, jamais Kind ni migration Compose. Ne pas réveiller un runtime arrêté pour du travail documentaire/statique ; aucun démarrage de Kind pour l'E2E avant la fin des corrections et de la phase IT finale (la compilation Docker / pure unit gates en cours d'implémentation reste autorisée — un slot cargo, ledger propre).
- Le parent tient l'inventaire AVANT et le bail partagé des IDs créés/redémarrés explicitement pour la tâche ; ni nom ni image ne prouvent la propriété. Les workers rendent cet inventaire sans arrêter un runtime encore requis par le parent.
- Avant réponse finale, succès ou échec : arrêter gracieusement les ressources possédées devenues inutiles (permission permanente pour ce stop réversible, annoncer périmètre/perte de mémoire volatile), vérifier les fuites ; aucune suppression/prune/reset. Rétention seulement pour travail réellement actif/bail parent ou demande explicite de keepalive.
- Pour libérer la RAM, arrêter Docker Desktop puis `wsl --shutdown` seulement sans autre workload/bail, propriété inconnue ni cluster protégé affecté ; sinon signaler la RAM résiduelle et demander un arbitrage ciblé. Vérifier états + vmmem/RAM disponible sans Docker CLI après shutdown.
- Persister un petit ledger non secret avant les opérations longues ; signaler tout nettoyage bloqué. Politique de prompts, pas de moniteur d'inactivité installé : déconnexion brutale/kill ne garantit pas le teardown. Ne pas interrompre git ni un publisher hôte en arrière-plan.

## Problèmes infrastructure

Routage des diagnostics locaux (Docker Desktop, Compose, Kind `aiforall-local` et `aiforall-local-full` — liste close, Envoy du dépôt). Pas de prod. Jamais `kind-apparatus-p4-it` ni `rancher-desktop` ni tout autre contexte non listé.

- **Docker / Compose** → `container-runtime-debugger` (skills `.agents/skills/docker-*`).
- **Kubernetes / Kind** → `k8s-operator` (read-only par défaut).
- **Envoy / routing / TLS / networking** → `envoy-network-debugger` (skill `envoy-runtime-debug`).
- **Diagnostic kernel / TCP / DNS** → Inspektor Gadget MCP **si** déployé in-cluster et Ready ; sinon INCONCLUSIVE.
- **Après un changement infra significatif** → `infra-verifier` (PASS/FAIL/INCONCLUSIVE, ne modifie rien).

Parallèle autorisé. Exemple HTTP 503 : `envoy-network-debugger` + `k8s-operator`, puis `infra-verifier`.

Règles toujours appliquées : `.cursor/rules/infra-safety.mdc`. Détail d'installation : `docs/cursor-infra-agents.md`.
