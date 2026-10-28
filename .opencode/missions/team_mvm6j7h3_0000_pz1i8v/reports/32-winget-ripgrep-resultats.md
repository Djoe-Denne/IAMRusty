# 32 — Winget accessible et ripgrep installé

## Résultat
Demande confirmée via le choix **Winget** (gestionnaire puis ripgrep).
Travail terminé en portée utilisateur, sans élévation ni redémarrage des MCP.

- **Winget v1.29.380** était déjà installé avec App Installer ; l'absence de
  commande venait du PATH, pas d'un package manquant. Pas de réinstallation.
- Entrée `%LOCALAPPDATA%/Microsoft/WindowsApps` ajoutée au **PATH utilisateur**,
  avec sauvegarde de la valeur précédente. Toutes les anciennes entrées restent
  présentes ; **PATH système inchangé**, vérifié après installation.
- **Ripgrep15.2.0** installé par Winget, paquet `BurntSushi.ripgrep.MSVC`,
  source communautaire `winget`, `--scope user`, portable Windowsx64MSVC.
- `rg` fonctionne aussi dans le **Context Mode déjà lancé**, sans modifier son
  processus ou redémarrer le backend Rust/8B.

Continuité : [décision30](30-cible-installation-windows.md),
[préflight31](31-winget-ripgrep-preparation.md).

## Installation et chemins
Commande exécutée via l'alias absolu Winget :
`install --id BurntSushi.ripgrep.MSVC --version 15.2.0 --exact --source winget --scope user --silent --disable-interactivity`.
Exit0 ; journal Winget indique hash vérifié, extraction et installation réussies.
Pas de sources autres que `winget` choisies, pas d'upgrade global.

Binaire principal géré par Winget :
`%LOCALAPPDATA%/Microsoft/WinGet/Packages/BurntSushi.ripgrep.MSVC_Microsoft.Winget.Source_8wekyb3d8bbwe/ripgrep-15.2.0-x86_64-pc-windows-msvc/rg.exe`.
Winget ajoute ce répertoire au PATH utilisateur.

Le premier script de contrôle supposait un lien `Microsoft/WinGet/Links/rg.exe`.
Ce lien n'est pas créé dans cette installation : son contrôle a donc échoué
**après** une installation Winget réussie. Inspection du journal et du package,
puis validation du binaire réel ; **aucune réinstallation/retry aveugle**.

## Accès immédiat depuis les processus existants
Une modification du PATH utilisateur ne met pas automatiquement à jour le PATH
hérité des serveurs déjà lancés. Copie de compatibilité créée, comme annoncé :
`C:/Users/djden/.local/bin/rg.exe`, répertoire déjà au PATH du sandbox.

Copie atomique sans écrasement d'un fichier préexistant (`File.Copy(..., false)`),
SHA256 identique au binaire principal Winget :
`14231169855ec5205cf5a1b6f1db358ff4aed4247c86b69ce8aae647c77f6680`.
Cette empreinte est celle de l'EXE, distincte du hash de l'archive vérifiée par Winget.

**Limite de maintenance : cette copie n'est pas automatiquement actualisée par
une future mise à jour Winget.** L'installation principale reste gérée par Winget ;
rafraîchir explicitement la copie après upgrade, sans écrasement aveugle d'un
autre binaire. Aucun wrapper, daemon ou hook de synchronisation ajouté.

## Vérifications réelles — toutes exit0
Au2026-10-24T14:57:08Z :
- CMD avec environnement enfant reconstruit depuis les PATH enregistrés :
  `winget --version` → v1.29.380 ; `rg --version` →15.2.0.
- PowerShell sans profil, même environnement : mêmes versions.
- Context Mode **sans remplacement de son environnement hérité** :
  `rg --version` →15.2.0 ; recherche stdin littérale numérotée → `2:needle`.
- `winget list --id BurntSushi.ripgrep.MSVC --exact --scope user --source winget --disable-interactivity`
  retrouve15.2.0, donc paquet effectivement enregistré, pas seulement EXE copié.
- Anciennes entrées PATH utilisateur toutes conservées ; PATH système identique.

Ces tests ne changent pas rétroactivement l'environnement d'une console déjà
ouverte : **ouvrir une nouvelle console pour bénéficier du PATH Winget**.
Aucun restart OpenCode/1MCP/Rust MCP n'a été fait ou n'est nécessaire pour rg
dans le sandbox actuel grâce au répertoire déjà au PATH.

## Preuves privées et retour arrière
Sous `%LOCALAPPDATA%/Temp/opencode/windows-tools-20261024/` :
- `environment-before.json` : anciens PATH utilisateur/système (ne pas dumper) ;
- `winget-install.log` : journal de l'installation ;
- `installation-result.json` : résultat Winget et premier contrôle de lien,
  conservé comme preuve de cette hypothèse erronée, pas preuve finale négative ;
- `compatibility-copy.json` : source/cible, propriété de la copie et SHA256.

Scripts inspectables sous `%LOCALAPPDATA%/Temp/opencode/` :
`install-winget-rg-20261024.ps1`, `expose-rg-20261024.ps1`.
Ces sauvegardes sont conservées ; aucun rollback/désinstallation déclenché.
Un éventuel rollback devra préserver les changements utilisateur ultérieurs,
pas remettre aveuglément tout l'ancien PATH.

## Périmètre et clôture
Aucun code applicatif, profil agent, index/graphe ou configuration MCP modifié.
Aucun cache modèle libéré, backend arrêté, Cargo/Docker/Kind/WSL lancé ou
ressource Docker créée. Installation durable demandée, pas une fixture à nettoyer.
Seuls package/outillage utilisateur, PATH utilisateur, copie de compatibilité et
preuves/rapports associés changés. Aucun accord bloquant restant.
