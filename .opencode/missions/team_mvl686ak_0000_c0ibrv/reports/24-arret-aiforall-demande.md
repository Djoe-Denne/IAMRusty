# 24 — Arrêt AIForAll demandé par l'utilisateur

Utilisateur : nouvelles découvertes susceptibles de changer l'approche ; demande
explicite d'arrêter le calcul. Aucun nouveau chantier ni reprise admis.

Préflight 20:11:39Z : watcher 51668, génération 17:22:12.5511750Z, toujours possédé ;
moniteur 14780, génération 20:04:07.5933610Z, lock correspondant. État `warming` /
`running`. Contrôles PID, création et commande concordants.

Exécution bornée : arrêter uniquement le moniteur vérifié, archiver son lock ;
sauvegarder les fichiers persistés avant arrêt natif gracieux du watcher, puis
sauvegarder ce qui est persisté après arrêt. Les snapshots existants restent.
Une erreur de sauvegarde ne doit pas empêcher l'arrêt demandé.

Pas de promesse de récupération de tout le travail en RAM ni de graphe complet.
Préserver index, credentials, cache et proxy LiteLLM partagé ; pas de nouvel appel
génératif, de rebuild, de restauration ou de redémarrage. Les preuves de protection
des rapports 22–23 restent valides. Aucun Docker/Kind/WSL/Cargo requis.
