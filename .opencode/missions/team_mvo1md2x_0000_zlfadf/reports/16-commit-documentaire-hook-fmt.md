# Rapport 16 — commit documentaire bloqué par le hook fmt

## Objectif et état

T1 a rédigé le contrat (204 lignes) et deux index. Aucun commit créé : le hook
versionné lance cargo fmt --all -- --check et échoue avec erreur Windows 206
selon le membre. Ce lancement incidental fmt n'est ni un test exécuté vert ni
un changement applicatif ; aucun build/test/runtime n'est annoncé.

## Preuves du lead

Lecture intégrale du contrat dans le worktree architecte. Git exécuté depuis
ce même worktree : staged contient uniquement docs/adr/README.md,
docs/contracts/README.md et docs/contracts/telegraph-notification-outbox.md,
3 fichiers / 209 insertions. git diff --cached --check : exit 0, aucune sortie.
core.hooksPath=.githooks ; son pre-commit contient exclusivement set -eu puis
cargo fmt --all -- --check. Aucun filtre de fichiers documentaires.

## Options et recommandation

1. Autoriser, pour ce seul commit des trois Markdown vérifiés, un override
   core.hooksPath vers un répertoire vide contrôlé via git -c, sans git config
   persistant et sans changement du hook. Recommandé : le hook Rust n'apporte
   aucune validation au contenu modifié ; les contrôles documentaires passent.
2. Refuser cette exemption : conserver le staged et résoudre le problème
   cargo fmt/hook dans un périmètre de réparation séparément autorisé avant
   commit. Ne pas annoncer T1 intégrée ni libérer T2 entre-temps.

## Décision demandée

Autoriser l'exemption ponctuelle de hook pour le seul commit documentaire,
avec recontrôle préalable des chemins staged et du diff --check. Aucune
exemption pour les commits Rust à venir, aucun changement durable de politique.
