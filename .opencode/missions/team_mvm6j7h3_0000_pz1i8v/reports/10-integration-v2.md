# 10 — Version corrective intégrée, vérification réelle en cours

## État / changements depuis 09
- Sept commandes natives de préparation terminées exit 0 (sécurité, chunks,
  identité, audit réel, paths, HIR, build), toujours un slot Cargo et 3 jobs.
- Binaire v2 côte à côte actif, SHA256
  `77e4936d794b54513a73cbfbc4405de05c56e796deea78541cd1347276046e41` ;
  backend vérifié PID 45784. v1/upstream conservés.
- Sauvegarde des données pré-v2 : **310 fichiers copiés et SHA256 recoupés**,
  preuve `data-backup-v2.json` privée. Ni modèle ni ancien index effacés.
- Overlay complet exporté sans staging : 17 fichiers, y compris le nouveau
  module de redaction ; `patches/workspace-corrections-v2/manifest.json`.
- Audit source final : 914/914 indexables, 52 vues à expurger. La nouvelle règle
  détecte aussi un cas dans les sources antérieurement indexées ; son ancienne
  vue sera remplacée, pas conservée en doublon. Les vecteurs non affectés restent.

## Preuves complémentaires
- QMD MCP : status et recherche lexicale AIForAll réussis, document RustyCog
  réellement récupéré. 275 documents dans la collection projet, aucun embedding
  manquant déclaré. Aucun update/rebuild du wiki lancé.
- Obsidian CLI présent mais application fermée : `vaults`/`help` ne peuvent pas
  joindre Obsidian. Ce n'est pas une validation du runtime de l'application ;
  aucun démarrage forcé ni modification du vault pour contourner ce constat.
- GrepAI lifecycle : AIForAll service/backend/watch arrêtés ; les services ARTIST,
  retro-eng et autres projets restent disponibles. Catalogue OpenCode sans GrepAI.
- JSON projet/1MCP valides ; 13 fichiers TOML Codex parsés. Profils écrivains
  des trois clients mis à jour, permissions/modèles non changés.
- Les 989 sources/manifests/locks du manifeste initial sont inchangés/présents.

Preuves privées : `qmd-migration-proof.json`, `qmd-document-read-proof.json`,
`migration-static-verification-v2.json`. CLI `opencode` absent du PATH de ce
shell : pas de preuve `opencode mcp list`, la connexion réelle des outils et le
catalogue de la session constituent la preuve client disponible.

## Vérification active / limites
`verify-workspace-v2.mjs` lancé sous shell sh_16dcbfedf001CyU2eUgmUkfl7h : graphe
réel forcé après backup, oracles callers/callees/codemap, replay source ciblé,
alias Windows puis fixture edit/rename/delete/recréation. Attendre notification,
pas de deuxième index concurrent. Résultats persistés au fil de l'eau dans
`workspace-v2-verification.json` ; ne pas annoncer PASS avant son résultat.

Un relevé de sécurité mémoire pendant ce run : 19471 MiB VRAM utilisés,
4852 MiB libres, GPU global 95 %. Ce n'est pas une médiane ni une mesure par PID.
Pas de Docker/Kind/WSL lancé ou créé. Pas de décision supplémentaire requise.
