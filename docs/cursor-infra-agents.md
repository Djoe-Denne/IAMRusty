# Agents infrastructure locale (Cursor)

Périmètre : **local uniquement** (Docker Desktop, Compose, Kind `aiforall-local`, Envoy du dépôt, CI GitHub du dépôt). Pas de prod.

## Composants installés

| Composant | Version / emplacement | Rôle |
|---|---|---|
| Docker Engine | 29.4.0 | Runtime hôte |
| Docker Compose | v5.1.2 | Stack locale |
| kubectl | v1.34.1 (cluster Kind v1.32.2) | Lecture Kind |
| Kind | v0.27.0 | Cluster `aiforall-local` |
| Node / npm / npx | v24.11.1 / 11.6.2 — `C:\Program Files\nodejs` | MCP npm |
| `kubernetes-mcp-server` | **0.0.67** (npm, pas `@latest`) | MCP K8s read-only |
| `ig-mcp-server` | **v0.0.1-beta.5** windows-amd64 | MCP Inspektor Gadget |
| Binaire IG | `%LOCALAPPDATA%\Programs\ig-mcp-server\ig-mcp-server.exe` | Hors dépôt |

CLI `npx skills add` : skills **v1.7.0**. Helm CLI : **absente** (volontaire).

## Skills

Le CLI Cursor installe les skills **projet** sous **`.agents/skills/`** (pas `.cursor/skills`). Emplacement **conservé**. Les skills RustyCog / AIForAll existants dans `.cursor/skills` et `.agents/skills` n'ont **pas** été déplacés.

Skill maison (chemin demandé) : `.cursor/skills/envoy-runtime-debug/`.

| Skill | Source | Chemin |
|---|---|---|
| docker-compose-patterns | docker/skills | `.agents/skills/docker-compose-patterns` |
| docker-build-strategies | docker/skills | `.agents/skills/docker-build-strategies` |
| docker-destructive-guardrails | docker/skills | `.agents/skills/docker-destructive-guardrails` |
| kubernetes | iuliandita/skills | `.agents/skills/kubernetes` |
| kubernetes-health | iuliandita/skills | `.agents/skills/kubernetes-health` |
| networking | iuliandita/skills | `.agents/skills/networking` |
| observability | iuliandita/skills | `.agents/skills/observability` |
| debug-triage | iuliandita/skills | `.agents/skills/debug-triage` |
| ci-cd | iuliandita/skills | `.agents/skills/ci-cd` |
| envoy-runtime-debug | ce dépôt | `.cursor/skills/envoy-runtime-debug` |

Lockfile : `skills-lock.json` à la racine.

## MCP

Fichier fusionné : `.cursor/mcp.json` (context-mode, grepai, serena **conservés**).

| Serveur | Transport | Config |
|---|---|---|
| kubernetes-mcp-server | stdio | `npx-cli.js -y kubernetes-mcp-server@0.0.67 --config` → `.cursor/kubernetes-mcp.toml` ; `--log-file stderr` ; `--disable-multi-cluster` |
| ig-mcp-server | stdio | `ig-mcp-server.exe -read-only -context kind-aiforall-local` |

TOML K8s : `read_only = true`, `toolsets = ["core", "config"]` (pas helm), `[[denied_resources]]` Secret `v1` group vide.

Flags vérifiés : `kubernetes-mcp-server@0.0.67 --help` (`--config`, TOML `read_only` / `toolsets` / `denied_resources`) ; `ig-mcp-server.exe --help` (`-read-only`).

## Subagents (`.cursor/agents/`)

| Agent | Fonction |
|---|---|
| k8s-operator | K8s/Kind fichiers, pods, deploy, svc, EndpointSlice, ConfigMap, events, logs, rollouts. READ ONLY par défaut. Destruction → parent. |
| envoy-network-debugger | Envoy, listeners, routes, clusters, endpoints, TLS/SNI/H2/DNS/Service. Skill envoy-runtime-debug. |
| container-runtime-debugger | Docker/Compose, healthchecks, logs, networks, ports, volumes, build. Jamais prune / `down -v` / `rm -f` spontané. |
| infra-verifier | Après changement : Docker, K8s, Envoy, chemin réseau. PASS/FAIL/INCONCLUSIVE. Ne modifie rien. |

Routage : `AGENTS.md` section **Problèmes infrastructure**. Règle : `.cursor/rules/infra-safety.mdc`.

## Fonctions MCP Kubernetes (toolset core+config, read-only, 0.0.67)

Smoke stdio réel : `configuration_view`, `events_list`, `namespaces_list`, `nodes_log`, `nodes_stats_summary`, `nodes_top`, `pods_get`, `pods_list`, `pods_list_in_namespace`, `pods_log`, `pods_top`, `projects_list`, `resources_get`, `resources_list`. **Aucun** outil Helm.

Écriture cluster refusée (`read_only`). Secret `v1` dans `denied_resources`. Pas de `nodes_list` dédié : nodes via `resources_list` / `nodes_top`.

Relancer Cursor après fusion MCP pour que l'IDE charge les nouveaux serveurs.

## Volontairement non installé

- docker-agent-*, docker-sandboxes-*, docker-project-foundations
- iuliandita : terraform, ansible, databases, docker, privilege-escalation, et le reste du catalogue (62 skills)
- Helm CLI ; toolset `helm` du MCP K8s
- Déploiement in-cluster Inspektor Gadget (nœud Kind **NotReady**)

## Limites Windows / Docker Desktop

- Au moment de l'installation : nœud `aiforall-local-control-plane` **NotReady** (`KubeletNotReady`, container runtime down). Pendant les smoke tests : conteneur Kind **Exited (137)**, API `127.0.0.1:60787` refusée. Ne pas le redémarrer depuis ces agents.
- Noyau du node Kind (quand il tournait) : `6.6.87.2-microsoft-standard-WSL2`, containerd 2.0.2. Docker Desktop 4.70.0 : containerd v2.2.1, runc 1.3.4.
- Un `.exe` `target\` Windows ne démarre pas dans Kind. Compiler les images dans Docker, pas `cargo` hôte.
- Ne pas partager `target\` hôte avec un build Linux.
- Admin Envoy Compose : bind `0.0.0.0:9901` **dans** le conteneur (`ops/deploy/mesh/envoy.yaml`) ; `docker-compose.yml` publie `10000:10000` seulement. Overlay Kind : `127.0.0.1:9901`. Ne pas publier l'admin.
- Clusters aussi présents : `apparatus-p4-it`, contexte `rancher-desktop` — **hors périmètre**.

## Commandes de vérification

```powershell
docker version
docker compose version
docker ps
docker compose -f docker-compose.yml config --quiet
kubectl config current-context
kubectl --context kind-aiforall-local get nodes
kubectl --context kind-aiforall-local get ns
kubectl --context kind-aiforall-local get pods -A
kind get clusters
npx skills list
& "$env:LOCALAPPDATA\Programs\ig-mcp-server\ig-mcp-server.exe" -version
```

Gadget live : uniquement si DaemonSet gadget **Ready**. Envoy admin : uniquement si une instance tourne déjà.

## Mise à jour des skills

```powershell
npx skills add --help
npx skills update --project
```

`--project` (`-p`) limite aux skills projet. Ne pas passer `--global` pour cet environnement. Après update, relire `skills-lock.json`. Pour le MCP K8s : changer **explicitement** `kubernetes-mcp-server@0.0.67` dans `.cursor/mcp.json` (jamais `@latest`). Pour IG : retélécharger le tag GitHub voulu vers `%LOCALAPPDATA%\Programs\ig-mcp-server\` et pinner le tag dans la doc + `mcp.json`.

## Inspektor Gadget in-cluster

**SKIP** : nœud pas Ready (puis conteneur Kind Exited). Ne pas réparer le nœud. Méthode prévue (non exécutée) : `kubectl apply` d'un manifeste upstream **pinné** (pas Helm), contexte `kind-aiforall-local` seulement. MCP IG : outil `ig_gadgets` ; gadget live **INCONCLUSIVE** tant que le DaemonSet n'est pas Ready.
