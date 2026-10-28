# 03 — Installation MCP et Qwen3-8B GPU : gate initial réussi

## Résultat borné

rust-code-mcp est installé nativement sous Windows et activé pour OpenCode dans
AIForAll. Le modèle 8B a calculé de vrais embeddings CUDA ; appels MCP réels
depuis cette session. Ce n'est PAS la clôture de toute la mission de migration.

## Configuration finale de cette étape

- Ajout de cinq lignes dans `opencode.jsonc` : serveur distant rust-code-mcp,
  `http://127.0.0.1:3062/mcp?tags=rust-code-mcp`.
- Backend ajouté seulement au profil 1MCP `projects/aiforall/mcp.json` : lanceur
  `C:/Users/djden/AppData/Local/rust-code-mcp/start-mcp.mjs`, marqueur v1.
- Binaire/commit/build : voir rapport 02 et `installation.json` externe.
- 55 outils accessibles par Code Mode, sans injection native des 55 signatures.
- Second client HTTP et OpenCode : même PID 62712. Reload ciblé backend : PID
  54544, ancien remplacé, un seul rust-code-mcp observé ensuite. Aucun restart
  global du client ou de 1MCP. Redémarrage du PC non effectué.
- GrepAI reste actif provisoirement. Serena, QMD, Obsidian et tous les autres
  objets MCP préexistants sont inchangés (comparaison des sauvegardes PASS).

## GPU, modèle et preuves

- Profil explicite local-qwen3-8b ; backend local-qwen3-candle-cuda, F16,
  dimensions 4096. Logs : loading Qwen3 model ... device=Cuda, puis initialized.
- Snapshot officiel Qwen/Qwen3-Embedding-8B :
  `1d8ad4ca9b3dd8059ad90a75d4983776a23d44af`, quatre shards / 15134634568 octets ;
  les quatre SHA256 correspondent aux métadonnées LFS du dépôt officiel.
- Rectification du cache envisagé au rapport 02 : HF_HOME est ignoré par le
  loader ApiBuilder::new(). Cache RÉEL conservé, hors dépôt :
  `C:/Users/djden/.cache/huggingface/hub/models--Qwen--Qwen3-Embedding-8B`.
  Aucun déplacement ni duplication manuelle des poids, aucune donnée HF supprimée.
- Sonde : un fichier, trois chunks ; trois embeddings en 2,56 s hors chargement,
  première indexation 179 s incluant téléchargement/chargement. Recherche
  descriptive : divide_work en premier, plausible d'après la source contrôlée.
- 44 relevés GPU : baseline totale 5724 MiB, pic total 21360 MiB, minimum libre
  2963 MiB. Le delta global est de l'ordre de 15 Go, PAS une mesure par PID.
- Pas de CPU/API distante pour les embeddings de cette sonde. Le backend local,
  le dispositif CUDA et les vecteurs produits sont vérifiés, pas juste déclarés.

## Client réel et persistance : acquis / limites

- Appels depuis OpenCode : runtime_status, index_codebase, search,
  get_dependencies et clear_runtime. Analyse d'un vrai fichier AIForAll : les
  douze imports de `crates/readiness/src/probe.rs` correspondent à sa source.
- Vecteurs réutilisables après arrêt propre puis nouveau backend ; recherche
  réussie après reload. Les index vivent sous
  `C:/Users/djden/AppData/Roaming/rust-code-mcp/search/data/`.
- Réserve réelle : alias de chemins Windows C:/ et C:\ -> hashes de stockage
  différents (`indexing/project_paths.rs::dir_hash` utilise le chemin lexical),
  tandis que l'identité Merkle canonicalise. Une recherche a donc dû indexer
  une seconde représentation de la fixture. Aucun cache supprimé.
- Après reload, un autre index_codebase a annoncé absence de snapshot, chargé
  le modèle et reparcouru le fichier, puis `Parser error: File unchanged` :
  zéro nouveau chunk/embedding, mais ce n'est PAS une reprise Merkle rapide
  entièrement validée. Frontier : incremental.rs / project_paths.rs / identity.rs.
- Graphe, workspace complet et tests incrémentaux A–E restent à effectuer.
  La stabilité VRAM n'est acquise que sur trois chunks courts ; batch 32 par
  défaut non validé sur 32 chunks longs, à borner avant indexation massive.

## Preuves persistées et ressources

Sous `C:/Users/djden/AppData/Local/rust-code-mcp/` :

- `mcp-gpu-smoke-result.json` et son stderr (campagne initiale, exit 0).
- `model-verification.json` (quatre SHA256 PASS).
- `http-sharing-proof.json` (second client, même PID).
- `integration-proof.json` (configuration minimale, device CUDA, reprise).
- `server-runtime.json` et `logs/server-2026-10-24T09-36-35.525Z-54544.stderr.log`.
- `README.md` (reproduction, limites, chemins effectifs et rollback).

La mémoire du cache de recherche de la fixture a été libérée via clear_runtime,
sans suppression d'index. Après : 5890 MiB utilisés / 18433 MiB libres au relevé.
Le seul backend installé reste disponible et recharge le modèle à la demande.
Aucun Docker/Kind/WSL lancé, aucun conteneur créé, aucun Cargo encore requis.

## Rollback / suite

Originaux vérifiés dans `.../rust-code-mcp/backups/mcp-config-20261024-113104/`.
Retirer uniquement les deux entrées rust-code-mcp ajoutées, après comparaison
avec changements ultérieurs ; ne pas écraser globalement les configurations.
Ne supprimer ni modèle/cache, ni données GrepAI/QMD.

Suite : fiabiliser/baliser chemins et reprise, configurer les batches pour la
VRAM disponible, puis indexation/graphe du vrai workspace et routage déterministe.
Les profils d'écriture Serena et la désactivation locale GrepAI restent prévus,
non appliqués dans ce gate d'installation.
