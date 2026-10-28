# Rapport 11 — préflight Git T1 vérifié par le lead

## État et différence depuis le rapport 10

Le membre confirme que le déplacement natif a rendu .git et les index ADR et
contrats lisibles dans son worktree. Le seul refus résiduel de ce préflight
vise le HEAD du gitdir partagé, situé hors de son périmètre external_directory.

## Preuve vérifiée

Le lead a exécuté via Context Mode quatre commandes Git depuis
`C:/Users/djden/.local/share/opencode/worktree/186749/ensemble-04d9a2f8-zlfadf-t1-architecte` :

- rev-parse --show-toplevel : confirme exactement cette racine ;
- rev-parse HEAD : `93e376c5530a439aaafe61a64d3e881865c1acf1` ;
- symbolic-ref --short HEAD : `ensemble-efe74926-2262-4e7e-bb66-f51c7b081bd1` ;
- status --porcelain : vide, arbre propre.

Quatre exit 0, stderr vide. Aucun fichier modifié par ces commandes.

## Suite et limites

La même session architecte a reçu ces résultats et un GO explicite de reprise.
Le prérequis HEAD vérifié est satisfait par cette preuve du lead, sans exiger
une lecture interdite redondante par le membre. Aucun droit élargi, aucun
gitdir manipulé, aucun bypass de son refus. Le lead prendra en charge le
diff --check si le profil documentaire ne permet pas le shell.

Analyse/rédaction et transition native de phase restent à exécuter par le
membre ; aucun contrat livré, test Cargo ou runtime lancé. T2 reste bloquée.
