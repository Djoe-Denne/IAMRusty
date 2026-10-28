# 31 — Winget puis ripgrep : préflight

## Autorisation et constat
L'utilisateur a choisi **Winget**, option explicitement décrite comme installation
du gestionnaire puis ripgrep. Travail direct, sans agent ni runtime de test.

Contrairement à l'absence de commande dans le PATH, le package utilisateur
`Microsoft.DesktopAppInstaller_1.29.380.0_x64__8wekyb3d8bbwe` est déjà présent,
statut0, sous Windows11x64. Invocation absolue de son alias WindowsApps :
`winget --version` → **v1.29.380**, exit0. Pas besoin de réinstaller App Installer.

## Changement minimal prévu
Ajouter seulement `%LOCALAPPDATA%/Microsoft/WindowsApps` au PATH **utilisateur**
s'il manque, en préservant toutes les entrées et en sauvegardant la valeur avant
mutation. Aucun PATH système ou paramètre MCP touché. Les processus déjà lancés
ne récupèrent pas automatiquement un PATH modifié : utiliser l'alias absolu pour
la suite des opérations et vérifier séparément l'accès depuis Context Mode.

Installer ensuite `BurntSushi.ripgrep.MSVC` depuis la source communautaire winget,
en portée utilisateur si le manifeste le permet. Vérifier les possibilités du
CLI et le manifeste avant installation. Ne pas désinstaller/remplacer une autre
version ni accepter une élévation/fermeture forcée implicitement.

Pour exposer rg au sandbox sans redémarrer les MCP/8B, privilégier une solution
dans un répertoire déjà au PATH, en refusant d'écraser un fichier préexistant.
Le mécanisme concret dépendra de l'emplacement de l'installation portable.

## Vérification et limites
Version Winget absolue, résolution avec PATH utilisateur mis à jour dans un
processus enfant, version rg en console et depuis Context Mode. Garder les
artefacts de rollback et les versions/emplacements effectifs. Pas de promesse
que le PATH d'un processus existant change sans intervention.

Aucune installation/purge/restart de MCP, index, modèle ou framework. Aucun
Docker/Kind/WSL ni compilation applicative. Pas de décision bloquante à ce stade.
