# Tests end-to-end

Le circuit E2E conservé est [k6](k6/AUTH-WINDOWS-PARENT.md), lancé par
`k6/run-windows.ps1` en mode `auth` avec sa configuration privée admise.
`auth-inspect` inspecte les scénarios sans les exécuter.

Les anciens pilotes E2E Python, shell et navigateur autonome ont été retirés
sur [décision utilisateur](../../docs/missions/20261106-k6-only-cleanup.md).
Les tests Python présents ici valident l'outillage : ils ne sont pas une autre
campagne E2E applicative. Les auxiliaires Python du parent k6 restent nécessaires.

Le raccord k6 au profil GitHub seul et la parité des anciens cas restent à
qualifier. Aucun fallback automatique vers l'ancien moteur n'est disponible.
