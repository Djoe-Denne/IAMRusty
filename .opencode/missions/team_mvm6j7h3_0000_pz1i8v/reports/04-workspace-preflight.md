# 04 — Lancement du vrai workspace : préflight

## Mandat et état

Utilisateur demande explicitement de lancer graphe et index du vrai workspace.
Exécution séquentielle prévue : index 8B, puis graphe HIR, pas de double pression
GPU/RAM. Aucune modification du code applicatif ni nettoyage des anciens index.

## Vérifications

- Répertoire unique pour les appels : `C:/Users/djden/source/repos/AIForAll`.
- Base Git 93e376c5530a439aaafe61a64d3e881865c1acf1, changements utilisateur conservés.
- `cargo metadata --locked --all-features --no-deps --format-version 1` PASS :
  65 membres, 77 cibles lib/bin/proc-macro candidates. Cargo.lock SHA256
  `e2f048e386e7eed2e9d4dc166528ff409497a225312fd8e8612afa7d94231bee`, inchangé.
- Aucun Cargo/rustc actif au préflight ; un backend MCP natif partagé.
- RAM disponible ~9,2 Gio, VRAM libre 17875 MiB ; C ~88 Go, E ~546 Go libres.
- Loader HIR upstream : toutes features, pas de build/check des out_dirs, pas de
  serveur proc-macro ; graphe normal lib/bin, couverture des macros limitée.

## Blocage évité avant indexation

Le walker upstream ne respecte pas gitignore et inclut `.tmp` : 1943 fichiers
Rust candidats, dont 1029 copies historiques sous `.tmp`. Les indexer polluerait
recherche/symboles avec d'anciennes versions. Corpus hors `.tmp` : 914 fichiers
(services 720, workers 72, rustycog 65, crates 48, runtime 7, docs 2).

Décision bornée : overlay local de l'outil, pas de l'application, pour exclure
`.tmp` et `.worktrees`. Filtrage identique dans le walker d'index et Merkle ;
empreinte du graphe/codemap alignée. Tests ciblés ajoutés, patch exportable,
ancien binaire conservé. Pas de suppression ni déplacement des sauvegardes.

## Ressources / lancement

Options upstream vérifiées dans rmc-config :
`RUST_CODE_MCP_EMBED_BATCH_SIZE=1`,
`RUST_CODE_MCP_EMBED_MAX_TOKENS_PER_BATCH=1024`.
Pas de changement des chunks (768/1024), du modèle 8B ou des dimensions 4096.
Rayon=2 pour le parsing ; un seul slot Cargo pour la préparation de l'outil.
Réglages applicatifs globaux inchangés. Aucun Docker/Kind/WSL requis.

Préparation : `.../rust-code-mcp/prepare-workspace-binary.ps1` (tests des deux
walkers, test d'empreinte, puis build). Nouveau binaire côte à côte ; déploiement
du backend seulement après tests/build PASS. Ensuite lancement MCP avec mesures
et état persistés ; aucun succès de campagne annoncé à ce stade.
