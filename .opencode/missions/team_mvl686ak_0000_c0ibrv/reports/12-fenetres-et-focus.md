# 12 — Fenêtres parasites : cause native identifiée

## Objectif / état
Supprimer les terminaux fugaces et pertes de focus, sans masquer l'impact sur
les trois enrichissements actifs. Aucun watcher ni proxy arrêté dans ce diagnostic.

## Preuves observées
- Une sonde WinEvent sans titres de fenêtres ni contenu utilisateur confirme
  **38 événements de premier plan Windows Terminal sur 6 secondes**.
- Snapshot Toolhelp natif, sans élévation : git.exe est lancé directement par
  chacun des daemons d'enrichissement : AIForAll47328, ARTIST45868, retro-eng46452.
  Apparition corrélée de conhost.exe/OpenConsole.exe.
- Les contrôles périodiques lancent aussi PowerShell, mais l'essentiel des
  lancements observés vient des git.exe enfants des daemons eux-mêmes. Ce n'est
  pas une fenêtre Luna/ChatGPT ni seulement une requête du moniteur.
- WMI ProcessStartTrace est refusé au compte courant : aucune élévation tentée.
  Diagnostic obtenu avec Toolhelp/WinEvent à la place, en six secondes bornées.

## Cause dans le code
GrepAI v0.37.0, `git/git.go:31,42,97` : exec.CommandContext("git", ...), sans
attribut Windows masquant les sous-processus. Les daemons sont détachés ; leurs
processus console Git peuvent créer un hôte de terminal et prendre le focus.
`windowsHide:true` utilisé par le wrapper Node ne se propage pas automatiquement
aux commandes que le programme Go lance lui-même.
Source : https://github.com/yoanbernabeu/grepai/blob/v0.37.0/git/git.go

## Options / recommandation
1. **Arrêter, corriger, relancer** les trois enrichissements : correctif Windows
   borné du lancement Git (HideWindow/CREATE_NO_WINDOW), vérifié avant remplacement
   du binaire. Peut nécessiter une toolchain Go isolée (Go absent du PATH actuel).
   Aucun changement de modèle, pas de cap ajouté, pas de modification du Git global.
   Les index persistés et cache LiteLLM sont conservés ; le calcul en mémoire non
   persisté devra être repris par un nouveau cycle. Recommandé vu le défaut de focus.
2. Reporter le correctif à la fin : conserve les calculs en mémoire, mais laisse
   le défaut de fenêtres pendant ces campagnes. Non recommandé pour l'ergonomie.

Pas d'injection dans les processus actifs, pas de changement des réglages globaux
de Windows Terminal, pas de patch du Git installé, pas de kill d'un terminal user.
Une promesse de correction des daemons déjà en mémoire sans relance ne serait pas
fondée ; aucun contournement de cette nature entrepris.

## Décision demandée
Autoriser l'arrêt temporaire des trois enrichissements, le correctif Windows
ciblé de GrepAI et leur relance, en conservant les index et le proxy/cache LiteLLM ?
