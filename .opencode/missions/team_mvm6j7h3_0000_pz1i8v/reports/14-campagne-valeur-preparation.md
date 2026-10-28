# 14 — Préparation d'une campagne de valeur, exécution directe

Demande : vérifier si le MCP fait gagner une information exploitable, pas seulement
s'il répond. Aucun nouvel agent, aucune reconstruction ni modification applicative.

Plan persisté avant exécution :
[`docs/missions/20261024-rust-code-mcp-value-plan.md`](../../../../docs/missions/20261024-rust-code-mcp-value-plan.md).
8 scénarios, 17 appels métier : découverte sémantique, voisins de refactor,
impact cross-crate, signatures/pagination, API publique, règles architecturales
synthétiques, audit de derive avec contrôle négatif, codemap orientée tâche.

Oracles ancrés dans les sources avant les réponses : trois sites d'appels de
factory réels, deux méthodes transport_status identiques vues par Serena,
deux signatures async &QueueConfig, réexports et attributs Debug connus.
Les règles de dépendance artificielles ne sont pas des findings du projet.

Préflight observé : backend41512, caches0/backgroundfalse, sources989 inchangées,
VRAM libre20975MiB. Pas de Cargo/Docker/Kind/WSL requis. Preuve privée
`value-campaign-preflight.json`. Durées froid/chaud et pertinence/limites évaluées
après run ; aucune approbation supplémentaire nécessaire dans ce périmètre.
