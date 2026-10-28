# 17 — 8B résident et commande manuelle : terminé

## Résultat
Le modèle8B est **chargé et volontairement conservé en VRAM**. L'utilisateur a
explicitement demandé ce keepalive ; ce n'est pas une fuite laissée sans bail.
Pas de passage4B, changement de dimensions, reconstruction ou réindexation.

Commande disponible dans **CMD et PowerShell**, depuis n'importe quel dossier :
- `aiforall-mcp-vram free` — libérer les caches volatils du backend Rust AIForAll.
- `aiforall-mcp-vram warm` — précharger le8B et conserver son cache serveur.
- `aiforall-mcp-vram status` — état du cache et relevé GPU global.
- `--json` disponible ; `--help` explique les effets. Action inconnue refusée.

## Implémentation et périmètre
Wrapper utilisateur `.local/bin/aiforall-mcp-vram.cmd`, déjà au PATH, vers
`%LOCALAPPDATA%/rust-code-mcp/vram-control.mjs`. Aucune modification du PATH,
profil shell, configuration globale de MCP ou installation de package.

Client HTTP ponctuel sur1MCP3062/tagRust ; vérification du workspace, PID et
emplacement du binaire AIForAll avant action. Il ne peut pas cibler une URL
arbitraire. free = clear_runtime mémoire du seul backend Rust vérifié, pas
clear_cache ; QMD/Serena/Obsidian et les données sur disque ne sont pas concernés.
warm exige l'index local8B/4096 existant et fait une recherche dense sans chemin
de reindexation automatique. Le processus CLI se termine après chaque action.

Le cache existant garde un Arc fort sans TTL : pas de nouveau daemon, timer,
watchdog, service Windows ou binaire Rust. La règle locale de keepalive est
inscrite en tête d'AGENTS, dans la règle Cursor et14 profils écrivains des trois
clients. Elle surcharge le cleanup générique de **ce modèle seulement**, pas
les garde-fous globaux ni la gestion de Docker/Kind/WSL/autres MCP.

## Vérification réelle
`resident-vram-verification.json` : **complete,21 assertions**.
- Résolution par CMD `where` et PowerShell `Get-Command`, depuis hors dépôt.
- Help fonctionne ; argument invalide retourne2 sans connexion/modification.
- PowerShell warm puis CMD status : cache8B=1 après disparition du premier CLI.
- CMD free puis PS status : cache8B=0, pas de rechargement provoqué par status.
- CMD warm puis PS status : cache8B=1 ; **état final conservé chaud**.
- Même backend PID41512 pendant tout le cycle, aucun kill/redémarrage.

Durées : premier warm34,4s, free449ms, dernier warm30,9s. VRAM **globale** :
18852MiB utilisés/5471 libres à chaud ; 2682/21641 après free, puis à nouveau
18852/5471 après warm final. Ces nombres ne sont pas une mesure d'allocation par PID.

**655 fichiers persistants** identiques avant/après (noms, tailles et SHA256),
**989 sources/manifests/locks** inchangés. Les13 TOML Codex restent valides.
Aucun Cargo/Docker/Kind/WSL démarré. Aucun code applicatif ou source du MCP changé.

## Limites et état à conserver
La résidence vaut tant que le backend/cache vit. Arrêt/crash/restart de Windows,
du pilote ou du backend libère les poids ; premier usage ou warm les recharge.
Après free, aucune boucle ne réchauffe le modèle, mais une prochaine requête
sémantique autorisée le rechargera. Des requêtes en cours peuvent retarder la
libération : free ne les interrompt pas de force.

Les règles agents sont des consignes locales explicites, pas un verrou technique
contre tout appel manuel à clear_runtime. Aucun hook global bloquant n'a été ajouté.
**Ne pas exécuter de cleanup automatique du modèle à la clôture de ce lot.**

Preuves privées : `resident-vram-verification.json`, `resident-vram-state.json`,
`resident-policy-static-check.json`. Documentation : `docs/rust-code-mcp.md` ;
ledger `.cursor/handoffs/20261024-rust-code-mcp-resident-vram.md`.
