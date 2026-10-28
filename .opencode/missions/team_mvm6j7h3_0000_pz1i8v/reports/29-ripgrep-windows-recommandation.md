# 29 — ripgrep Windows : vérification et recommandation

## Demande et résultat
L'utilisateur demande s'il peut installer `rg` et si cela vaut la peine.
Vérifications en lecture seule, **aucune installation ni modification du PATH**.
Windows est supporté par ripgrep ; son absence n'est pas une incompatibilité.

## Observations actuelles
- `spawnSync('rg', ['--version'])` depuis Context Mode : executable introuvable.
- PowerShell enfant ne résout ni rg/rg.exe ni winget dans cet environnement.
- `%USERPROFILE%/.cargo/bin/rg.exe` absent.
- `%USERPROFILE%/.local/bin` **et** `.cargo/bin` sont déjà au PATH du sandbox.
- L'alias `%LOCALAPPDATA%/Microsoft/WindowsApps/winget.exe` existe, même si
  `Get-Command winget` ne le retrouve pas dans le PATH hérité du sandbox. Cela
  ne prouve pas que winget soit indisponible dans une console utilisateur récente.

Source officielle consultée :
<https://raw.githubusercontent.com/BurntSushi/ripgrep/master/README.md>, sections
installation : binaire Windows statique disponible dans les releases ; commande
winget officielle `winget install BurntSushi.ripgrep.MSVC` ; installation Cargo
également supportée. Aucun téléchargement de binaire effectué.

## Recommandation proportionnée
Installer le binaire précompilé Windows x64 MSVC dans le répertoire utilisateur
déjà au PATH `.local/bin` évite compilation et changement d'environnement des
processus existants. Alternative si winget fonctionne dans la console utilisateur :
`winget install --id BurntSushi.ripgrep.MSVC -e`.
Vérifier ensuite `rg --version` dans la console **et** dans Context Mode ; succès
dans l'une ne garantit pas automatiquement le PATH de tous les services.
Pas de redémarrage global des MCP ni de libération du8B à proposer comme préalable.

Valeur : supprimer le détour des scripts qui appellent rg, disposer d'une commande
rapide pour le matching exact local. L'outil grep natif de l'agent fonctionne déjà,
donc c'est un confort utile, pas une condition pour Serena/Rust MCP ou une preuve
de gain important sur toute la mission. Les MCP restent prioritaires selon la tâche.

Aucune décision bloquante demandée, aucune installation autorisée implicitement.
Pas de backend/config/cache modifié, Cargo/Docker/Kind/WSL ou ressource créée.
