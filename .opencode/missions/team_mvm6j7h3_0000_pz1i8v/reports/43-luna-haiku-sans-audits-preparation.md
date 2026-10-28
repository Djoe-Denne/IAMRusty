# 43 — Luna high/xhigh et Haiku medium : préparation sans anciens audits

## Mandat et périmètre
Sur demande utilisateur, rendre les anciennes analyses de ce scénario indisponibles
à leurs chemins habituels avant trois nouvelles sessions natives, sans les détruire.
Code, documentation de conception, skills et configuration MCP restent inchangés.
Aucun briefing technique supplémentaire dans le prompt métier.

## Modèles vérifiés au catalogue
- openai/gpt-6-luna#high.
- openai/gpt-6-luna#xhigh.
- opencode-go/claude-haiku-5-5#medium, disponible et actif.
Pas de substitution par Sonnet, GLM ou un provider différent.

## Mise à l'écart prévue
12fichiers explicitement ciblés :8rapports de scénario/essais antérieurs,
3handoffs correspondants et1copie du prompt de mission. Déplacement réversible,
sans écrasement, avec inventaire/empreintes hors checkout ; pas de suppression
des autres travaux ni d'archive contenant des réponses placée dans le dépôt.
L'emplacement de récupération n'est pas publié ici pendant les essais pour ne
pas donner aux sessions un nouveau chemin vers les mêmes réponses.

## Méthode
Sessions build neuves, aucun parent, zéro message initial ; même prompt naturel
que précédemment, corps1162caractères, lecture seule et interdiction de tests/
infrastructure. Validation avant envoi : modèles, prompt, absence des12fichiers.
Les contrôleurs utilisent la pagination corrigée (cursor sans order).
Trois lancements en arrière-plan avec notifications, pas de polling, consignes
inchangées pendant les runs. Pas de rapport de résultats intermédiaire accessible
aux sessions encore en cours ; audit des traces seulement après la fin des3.

## Limites
Rendre les audits absents du filesystem réduit la fuite directe observée ; ce
n'est pas un confinement de sécurité. MCP/index/notes de conception partagés,
caches et charge concurrente subsistent. Ne pas supprimer ou réindexer les bases
de connaissances pour simuler un environnement totalement vierge.
Pas de mesure causale de vitesse/coût sur des profondeurs de raisonnement distinctes.

## État
Mise à l'écart effectuée :12fichiers déplacés, originaux absents et empreintes
des copies archivées vérifiées. Rien détruit. Journal privé durable conservé.
Les lanceurs et l'utilitaire de déplacement passent node--checkexit0.

Sessions créées et vérifiées build, aucun parent,0message avant envoi :
- Luna high : ses_e91558dc7ffebUY2lgBveuZiyq.
- Luna xhigh : ses_e91558c89ffeVJtGgIMUK6gG1V.
- Haiku Go medium : ses_e91558b46ffeWua15eoB36BzeU.

Lanceur : contrôle de nouveau l'absence des12anciens fichiers avant chaque prompt,
ne renvoie rien si la session n'est plus vide, conserve empreintes et état privés.
Les trois lancements suivent cet enregistrement ; aucune nouvelle analyse dans
le checkout avant leur fin. Aucun Cargo, testapp, Docker/Kind/WSL, restartMCP ou
déchargement8B requis.
