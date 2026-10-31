# 78 — Consolidation native des branches demandée

Objectif : intégrer les livrables durables des worktrees avant archivage, sans
ajouter leurs dossiers techniques ni réintroduire les anciennes propositions.

Depuis la tentative77, les12membres restants sont arrêtés. Onze arbres ont encore
du contenu non committé : documentation, instrumentation/configuration/tests,
et deux gitlinks dont les SDK imbriqués sont propres. La base principale reste
052817d, publiée. Le besoin n'est donc plus seulement de terminer l'administration :
l'utilisateur demande désormais le traitement explicite de ces branches.

Une préparation bornée des onze branches est indépendante de l'examen par le lead
de leur couverture dans master ; ces deux travaux seront menés en parallèle. Les
fusions natives sont ensuite séquentielles car elles modifient le même arbre.
Pas de modèle imposé, nouvelle infrastructure, nouvelle campagne ou nouveau GO.

Les contrats ratifiés priment : les versions plus récentes de master résolvent les
instantanés obsolètes, ADR0315 reste une alternative rejetée. Toute capacité utile
réellement absente sera identifiée avant correction et validation ciblée.

Contrat de travail persistant :
`.cursor/handoffs/20261029-worktree-consolidation.md`.
