# 02 — Binaire CUDA et protocole MCP validés

## Étape / delta depuis 01

Compilation Windows CUDA réussie à 3 jobs (exit 0), fin 2026-10-24T09:20:25Z,
après 4 min 15 s pour la reprise CRT ciblée. Pas de repli à 2 nécessaire.
Sources upstream et lockfile inchangés ; esaxx et Candle MoE désormais /MD,
confirmé dans les archives effectives par `dumpbin /directives`.

## Installation et preuves

- Exécutable installé :
  `C:/Users/djden/AppData/Local/rust-code-mcp/bin/rust-code-mcp.exe`.
- SHA256 : `4ee3e1c419ac6b9a0142769b8fbbf0f37c51de58fcf92ffc46152257bd3d2208`.
- Métadonnées : dossier d'installation `installation.json`, script
  `install-native.ps1`, logs `native-20261024-111611-103-j3.*`.
- Vrai échange stdio JSON-RPC : initialize puis tools/list, 55 outils ; arrêt
  runtime propre, exit 0. Répété via le lanceur installé `start-mcp.mjs`.
- Startup effectif : `RMC_EMBEDDING_PROFILE=local-qwen3-8b`,
  `CUDA-capable features compiled: true`, background sync désactivé.
- Résultats : dossier d'installation `mcp-probe-result.json` et son log stderr.

## Limites et prochain gate

Le catalogue MCP ne prouve pas un embedding GPU. Test isolé préparé dans
`probes/workspace` : index_codebase avec profil 8B explicite, puis recherche
descriptive et relevés nvidia-smi. Aucun index applicatif ne sera créé par cette
sonde, aucune logique applicative ne sera modifiée.

Modèle officiel Qwen/Qwen3-Embedding-8B : révision annoncée par Hugging Face
`1d8ad4ca9b3dd8059ad90a75d4983776a23d44af`, quatre shards, 15134634568 octets de
poids. Un cache isolé via HF_HOME était envisagé mais non encore vérifié.
Téléchargement et chargement GPU à valider à cette étape ; aucune réussite 8B
annoncée ici. Rectification après test : le loader ignore HF_HOME et utilise le
cache utilisateur HF par défaut, vérifié au rapport 03.

## Environnement préservé

Pas encore d'entrée ajoutée à OpenCode/1MCP. GrepAI, Serena, QMD et Obsidian
inchangés. Aucune modification PATH globale, aucun Docker/Kind/WSL lancé,
aucun conteneur créé. Installation et backups hors dépôt.
