# 01 — Installation native : blocage de liaison localisé

## Objectif / étape

Installer et prouver rust-code-mcp CUDA natif avant toute migration des MCP
existants. Compilation des crates et kernels atteinte, binaire non lié.

## Changements et faits vérifiés

- Sources upstream épinglées à `dfa9486e4d558cff7739658f645ae4c0f9d37e46`, hors
  dépôt. Protoc 36.2 officiel isolé et vérifié par SHA256.
- Préprocesseur CUDA/MSVC corrigé par option locale `/Zc:preprocessor`, sonde
  rouge/verte persistée. Builds Cargo à 3 puis 2 jobs atteignent la liaison.
- Les deux échecs sont LNK2038/LNK1319 : objets esaxx et trois objets Candle MoE
  compilés `/MT`, ORT compilé `/MD`. `dumpbin /directives` confirme ces directives.
- Sondes C++ et CUDA : options `/MD` produisent MSVCRT / MD_DynamicRelease au
  lieu de LIBCMT / MT_StaticRelease. Aucun `/NODEFAULTLIB` ni `/FORCE` proposé.
- Le cache Cudaforge ne compte pas les flags NVCC d'environnement dans son hash :
  relancer seulement Cargo pourrait réutiliser les mauvais objets natifs.

## Preuves

Racine externe : `C:/Users/djden/AppData/Local/rust-code-mcp/`.

- `logs/native-20261024-105023-077-j3.stderr.log`
- `logs/native-20261024-111006-694-j2.stderr.log`
- `probes/preprocessor-results.json`
- `probes/crt-results.json` (sondes CUDA/C++ MT puis MD)
- Ledger canonique : `.cursor/handoffs/20261024-rust-code-mcp-installation.md`.

## Choix borné

Recommandation appliquée : aligner réellement les CRT par flags de build locaux,
sauvegarder puis reconstruire les quatre artefacts Candle concernés, préserver
PTX et crates Rust. Pas de modification de registry Cargo ou sources upstream.
WSL reste une alternative si la voie native rencontre un blocage irréductible ;
pas de nouvelle distribution ou runtime démarré à ce stade.

## Inconnus / suite

Liaison complète, démarrage MCP et chargement 8B GPU non validés. Aucun MCP
existant désactivé/modifié, aucun conteneur créé, aucune décision humaine requise
pour cette correction locale réversible. L'escalade a conservé la même session
et le même modèle, sans nouveau worker ni seconde coordination.
