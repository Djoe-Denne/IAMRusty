# Diagnostic et replis bornés

Le contrat `AGENTS.md` reste normatif. Un repli conserve le périmètre, les
permissions et la QA ; il ne vaut pas permission de réparer l'infrastructure.

## Mauvais appel ou script
Consulter la signature réelle ; corriger argument, syntaxe ou échappement et
réessayer de façon ciblée. Une erreur du script Context Mode n'est pas une panne
du MCP. Respecter les délais connus : démarrage Serena potentiellement long,
chargement froid 8B autour de 30 s dans les mesures précédentes. Ne pas dupliquer
une requête encore en cours ni interpréter un chargement comme une panne.
Dans un appel groupé, une erreur peut masquer la réponse d'une autre lecture
sans l'arrêter côté serveur. Préserver les résultats avec `allSettled` ou un
traitement équivalent, vérifier chaque statut et attendre les appels lancés
avant de décider ce qui doit être relancé. Une erreur d'exécutable ou de script
n'est pas un résultat de recherche vide ; traiter sa sortie comme un échec.

Avant un nouveau script imbriqué complexe, vérifier si le skill local
`aiforall-context-functions` couvre le traitement. Un module réutilisé évite de
régénérer les échappements ; son chargement se fait **dans ctx_execute**, pas dans
le Code Mode confiné du client. Localiser les chemins inconnus avant batch,
valider ceux retenus et conserver les succès partiels. Pour rg, code 1 signifie
aucun résultat et code 2 une erreur ; ne pas confondre les deux. Extraire le
diagnostic sélectionné sans imprimer l'exception complète et son stdout.

## Lenteur sans panne établie
Comparer si possible temps client, planification/début de tâche et exécution
dans les journaux déjà disponibles. Une tâche longue peut retarder la suivante :
ne pas attribuer tout le délai à sa recherche ou à un cache défectueux. Une
lecture de corps et un inventaire transverse ne sont pas des opérations de coût
équivalent. Sans traces fines, dire « compatible avec une analyse froide » plutôt
que conclure à une corruption. Pas de retry parallèle, restart, purge ou
réindexation pour une seule latence élevée ; ne pas benchmarker hors mandat.

## Réponse vide / résultat incomplet
Vérifier racine effective, filtres, pages/totaux, exclusions, features/macros,
fraîcheur et diagnostics. Recouper avec Serena/HIR, sources et tests ciblés.
Sans preuve, dire « non retrouvé dans ce périmètre », pas « inexistant ».
Ni index vide, ni panne, ni besoin de reconstruction ne découlent du seul zéro.

## Fichier hors couverture / worktree
Comparer racine et version effectivement vues à celles des fichiers de la tâche.
`.tmp`/`.worktrees` sont exclus de l'index canonique. Utiliser les fichiers réels
du worktree et grep exact/patch précis ; distinguer résultat canonique et état
local. Ne pas réactiver le projet Serena partagé ou construire un index parallèle
sans mandat. Si la couverture ne peut être vérifiée, l'annoncer comme inconnue.

## Panne réelle du Rust MCP
Après un diagnostic ciblé et un réessai seulement si justifié, annoncer la panne
observée et utiliser Serena pour symboles/références, puis sources/tests ciblés.
Si la recherche comportementale manque, réduire la zone via les points d'entrée
connus et une inspection bornée ; pas de lecture massive du dépôt.

## Panne réelle de Serena
HIR Rust MCP et corps sources ciblés pour l'impact ; patch local précis si les
consommateurs/invariants peuvent être vérifiés. Si les deux MCP manquent, outils
fichiers bornés en explicitant la baisse de couverture. Différer une transformation
transverse invérifiable plutôt que substituer aveuglément du texte.

## QMD / Obsidian
QMD indisponible : recherche/lecture Obsidian si son CLI est opérationnel, avec
skills dédiés. Obsidian fermé n'empêche pas nécessairement QMD de répondre.
Si aucun accès ne permet la preuve ou l'édition wiki demandée, annoncer le manque
et demander un arbitrage. Pas de grep massif du vault, réindexation Rust du wiki
ou démarrage/réparation implicite d'une application.

## Context Mode réellement indisponible
Signaler le constat d'infrastructure puis **demander l'accord pour continuer sans
ce MCP**. Proposer un repli en lecture seule, sur commandes/chemins ciblés avec
filtrage et plafond de sortie avant retour au contexte ; pas de dump brut de logs,
documents ou secrets. Si confidentialité/taille/périmètre ne sont pas maîtrisables,
ne pas poursuivre ainsi. Une autorisation ponctuelle ne devient pas globale.

## Limites communes
Annoncer le repli une fois pour le périmètre concerné et consigner son impact.
Pas de polling/restart automatique, GrepAI implicite, modèle CPU/4B/API payante,
suppression de données, reset/reindex ou mutation de configuration globale.
Garder le 8B résident ; ne pas tenter de résoudre une panne en libérant sa VRAM.
Un refus de permission n'est pas une panne et ne peut pas être contourné.
