# 16 — Keepalive utilisateur8B et commande de contrôle

## Décision et périmètre
Utilisateur choisit explicitement de garder le8B en VRAM et demande une commande
CMD/PowerShell de libération manuelle. La proposition4B est abandonnée sans
téléchargement/reindexation ni changement de profil/dimensions/graphe.

## Solution bornée
Le cache serveur existant garde déjà les poids par Arc, sans TTL. Les décharges
antérieures étaient les clear_runtime de fin de tâche. Il suffit de conserver
ce cache et de rendre le contrôle manuel accessible ; aucun nouveau binaire ou
mécanisme de pooling/daemon n'est nécessaire.

Commande `aiforall-mcp-vram free|warm|status`, wrapper dans `.local/bin` déjà au
PATH (user et processus), script privé vram-control.mjs. URL fixe3062tagRust,
garde PID/workspace/binaire du backend AIForAll. free nettoie les caches volatils
de ce seul backend, pas QMD/Serena/Obsidian ni les données persistantes. warm
exige le vecteur8B/4096 existant et fait une requête dense, sans reindex automatique.

Exception locale de keepalive inscrite en tête d'AGENTS, dans la règle Cursor et
les profils écrivains OpenCode/Cursor/Codex. Elle prévaut sur le cleanup générique
de **ce modèle** seulement ; les autres garde-fous globaux restent inchangés.

## Vérification prévue
Résolution en CMD et PowerShell, help/erreur sans mutation, cycle warm/status/
free/status/warm ; clients CLI fermés entre appels, pour prouver la rétention
côté serveur. Comparer hashes des sources et des données persistantes, états
runtime et relevés GPU. **Laisser chaud à la fin**, selon le keepalive demandé.
Pas de Cargo/Docker/Kind/WSL ou modification du PATH/profil shell global.
