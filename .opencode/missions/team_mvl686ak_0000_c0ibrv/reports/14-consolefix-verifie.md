# 14 — Correctif Windows installé et vérifié

## Résultat
GrepAI installé : **0.37.0+windows-consolefix.1**, sans changement du protocole,
des index, du modèle ou des limites fournisseur. Proxy LiteLLM non redémarré.

## Localisation / impact
Source v0.37.0 au commit ad90c4216282a434eeb2e28df4a1ea175a5470aa, conforme aux
informations de build du binaire original (`vcs.modified=false`). Pas de correctif
local préexistant déclaré par ces informations à perdre.

Ajout `internal/executil` : exec.CommandContext avec **HideWindow** et
**CREATE_NO_WINDOW** sur Windows, comportement inchangé sur les autres OS.
Quatre sites Git remplacés : git/git.go (détection racine, common-dir, IsGitRepo)
et cli/watch.go (liste des worktrees). Aucun patch du Git installé, de Windows
Terminal, des processus en mémoire, de la logique RPG ou du parallélisme.

## Preuves
- 11 tests helper/Git PASS, dont un enfant réel GetConsoleWindow()==0,
  arguments inchangés, annulation, dépôt principal, worktree lié, Git absent,
  chemin invalide et stabilité d'identifiant.
- Tests CLI `TestDiscoverWorktreesForWatch*` PASS ; build Windows natif PASS.
- Sonde utilisateur non intrusive sur le candidat : **9 appels watch --status**
  dans AIForAll/ARTIST/retro-eng, tous exit0 ; **0 événement de changement de
  premier plan** sur 8 secondes, contre38événements/6s avant correction.
- Après installation : anciens index ARTIST lisibles via audit sur copie,
  coverageVerified:true ; sept catalogues1MCP14outils, managersv6 confirmés.

Artifacts : `C:/Users/djden/AppData/Local/grepai-windows-fix/`
(`candidate-focus-proof.json`, `deployment-proof.json`, `installation.json`, source).
Toolchain Go portable1.27.3, hash officiel vérifié, PATH global inchangé.

## Sauvegarde / retour arrière
Binaire original conservé à
`C:/Users/djden/AppData/Local/Programs/grepai/grepai.pre-consolefix-20261023-191917.exe`.
SHA256 ancien/nouveau consignés dans installation.json. Aucun fichier .old existant
supprimé. Une mise à jour future officielle peut écraser cette révision locale :
revalider le correctif alors, ne pas annoncer une correction upstream publiée.

## Suite autorisée
Relancer les trois enrichissements dans de nouveaux dossiers de campagne, avec
les mêmes racines et cacheproxy, puis vérifier une fenêtre de focus pendant
l'activité réelle. La preuve avant relance ne garantit pas tous les programmes
console Windows ni les lancements non-Git hors du périmètre de ce correctif.
