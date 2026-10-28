# 19 — Application du guidage MCP : préparation

## Objectif et autorisation
Appliquer la proposition du [rapport 18](18-proposition-guidage-mcp.md), puis
livrer un prompt utilisateur naturel à exécuter dans une nouvelle session.
Travail direct, sans agent supplémentaire. Ne pas exécuter la campagne à la
place de l'utilisateur et ne pas mentionner les outils dans son prompt.

## Périmètre
Contrat MCP dans AGENTS, skill projet unique et références, adaptateur Cursor,
rappels des profils locaux et documentation opérateur. Préserver les modèles,
permissions, procédures métier et restrictions de rôle de chaque profil.
Ni réglages globaux, ni backend, index, graphe, code applicatif ou runtime touchés.

## Constat nouveau
La duplication n'est pas uniforme : les profils rédacteurs/reviewers ont déjà
des rappels métier courts, les écrivains portent des sections MCP entières.
Conserver les indications métier utiles et remplacer uniquement la procédure
commune. Deux profils ADR Cursor imposent encore QMD CLI plutôt que MCP :
réconcilier avec la priorité MCP locale sans modifier les autres projets.

## Choix de sécurité
Les replis Rust/Serena sont autorisés dans leur périmètre de lecture/édition
contrôlé. En cas de panne réelle de Context Mode, garder l'accord explicite
avant poursuite sans ce MCP ; le repli proposé sera filtré et borné. Aucun
assouplissement implicite des permissions/sécurité par un fichier local.

## Vérification prévue
Contrôles statiques des références, frontmatters et TOML, préservation des autres
sections, absence de doublon du skill, contrôle des changements ciblés. La future
campagne observe une demande d'analyse métier, sans panne fabriquée ni exigence
d'appels précis. Les oracles restent séparés du prompt. Les capacités non
exercées ne seront pas comptées comme validées.

Aucun accord supplémentaire nécessaire pour avancer sur ce périmètre.
