# Rapport 65 — candidats SDK et adoption figés, pas encore validés

## Identités recoupées

SDK local **ed1809d145cfdb87be4ea1aaafb8dac7ae7967c9**, parent129dcc1575e7dd59b4c7935de0b3794dbb7e7d07, branche http-owned-c1 ; HEAD/status propre recoupés. Auteur annonce commit normal réussi, aucun push. Les neuf SHA256 du candidate-manifest.json correspondent aux sources SDK+AI réelles.

Adoption AI : tentative normale auteur unique bloquée Windows206. Lead applique la méthode éprouvée `.worktrees/hca`, contrôle scope/index/tree, puis commit normal **f2aa9e26cd1627ec5ab006a884e8fdebd15776b3**, arbre **8adb4400f80487af1cc4bc09f2ba0a664d613236** exact du candidat stagé. Hook normal réussi, +115/−65 sur cinq fichiers : lockAI, runtimehost, app/helper setup, fixtureRunningApp. Aucun test E1 ou pinSDK committé. Branche auteur actualisée par compare-and-swap après revalidation du snapshot, sans reset/stash/bypass.

## Conservation des preuves

Archive privée `.tmp/http-c1-candidate-evidence/manifest.json` du dépôt principal : neuf sources avec hashes/égalité octets vérifiée, quatre logs check, patch staged et manifeste auteur. Bundle SDK limité au commit candidat avec prérequis129dcc1 pour une matérialisation exacte et portable lors de la QA ; ni publication ni vendoring applicatif. Le manifeste de l'archive contient le hash du bundle.

Le check final annoncé par auteur après correction d'une course d'annulation planifiée reste à relier au snapshot dans la revue/tests ; les précédents checks ne deviennent pas des tests comportementaux. Aucun vert C-1/TLS/H2/streaming, Clippy ou Sonar nouveau acquis ici.

## Barrière d'intégration

pool-9 informé des commits et gel des sources, shutdown natif demandé. AIstatus auteur attendu : gitlinkSDK local modifié nonstagé et copie QA privée nontrackée, à préserver. Ne pas nettoyer de force ou assimiler ce checkout d'évaluation à un pin final publié.

Attendre shutdown effectif puis team_merge natif ; code livré n'est pas encore intégré au lead. Après intégration, matérialiser le SDK candidat exact pour QA sans committer le gitlink final, annoncer explicitement cette dépendance locale, puis publier test-engineer consommateur. Les compléments QA et le raccordement E1 vers l'API adoptée ne sont pas encore écrits/exécutés. C-1 demeure ouvert, pas de pin final avant revue/tests/publication du SHA exact.
