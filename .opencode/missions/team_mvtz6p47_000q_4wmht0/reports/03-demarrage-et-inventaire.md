# Démarrage autorisé — contrat Linux en cours

## Objectif et étape

La mission1 doit livrer un smoke réel simple sous Linux et qualifier Chromium séparément. L'utilisateur a approuvé la révision1 : OpenAI uniquement, long run, QA proportionate, dix commandes et seize critères. Le travail d'architecture est démarré nativement ; les trois consommateurs restent bloqués. Aucun code outillage ou runtime lancé par le lead.

## Organisation et limites

L'architecte possède ADR0504 et le contrat k6 dans son worktree isolé. Le lead examine en parallèle la provenance des outils et la CI, sans modifier les fichiers de l'architecte. Contrat ratifié et intégré avant auteur, puis revue indépendante ciblée et qualification intégrée. Le start est acquis : aucun nouveau GO à inventer aux jalons ordinaires. Une nouvelle frontière hors mandat reste à arbitrer.

Le laboratoire historique n'est pas adopté. Une cible Linux autorisée et joignable reste à établir ; l'absence de cible interdit le PASS HTTP/global mais ne bloque pas les travaux indépendants sur l'outillage et Chromium. Aucun déployeur, tunnel ou dispatch GitHub implicite.

## Préflight observé

Branche principale master, index vide, SDK02e5 inchangé. Le HEAD a avancé depuis la préparation uniquement sur les règles/documentation de clôture ; les sources des services et gardes examinées ne sont pas affectées. Les WIP étrangers Obsidian et SDK.serena/project.yml sont préservés.

Inventaire : 70 worktrees Git enregistrés racine comprise, 64 branches locales ; le nouveau worktree d'architecture est le seul arbre de cette mission observé. Les 69 autres arbres précédemment observés ne sont pas déclarés possédés par cette mission. Aucun ID Docker ou bail runtime créé/redémarré ; aucune opération de cleanup runtime à effectuer à ce stade.

## Traçabilité technique

- Policy révision1 approuvée, hash ce28526ae92efa825ad920cc157a62a5371c5fb32d1ed289639cbe5f7a627eba.
- HEAD et base auteur : 989614096110b9813ce67c47a23fad8457c2025a.
- Depuis c3c6596 : AGENTS.md, .cursor/rules/mission-closeout.mdc, docs/guides/livrables-durables-et-worktrees.md seulement.
- SDK : 02e5db23aac6016aab062c3078696d5f7fd464ef ; delta local préexistant .serena/project.yml.
- Auteur architecture natif pool-1, tâche task_mvtzpq0n_000t_82d0c4 ; branche ensemble-f2357db8-8d40-4117-9bb5-ecb795079323 ; worktree C:/Users/djden/.local/share/opencode/worktree/186749/ensemble-04d9a2f8-4wmht0-pool-1.
- Inventaire obtenu via git worktree list --porcelain, git branch --format, git status --short, git submodule status et comparaison des noms depuis la base de préparation. Pas de build ni de test.
