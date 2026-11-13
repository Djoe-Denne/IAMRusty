# Session CI, découverte fonctionnelle et k6 Ubuntu — 12–13 novembre 2026

Distillation de la session OpenCode `ses_e2f50a8f4ffegiKcR00QyAu8Tb` et de ses
quatre sous-agents accessibles. Source figée à la demande d'ingestion, pas à la
fin des opérations d'indexation. Inventaire récursif et empreintes :
[coverage.json](coverage.json). Contributions : [agents.md](agents.md).

## Couverture et confidentialité

L'export natif complet contient 1 519 messages avant la borne ; 287 enregistrements
de texte utilisateur/assistant en sont retenus pour lecture. Les 2 064 parties
de raisonnement, 1 538 parties d'outils, événements système/compaction et payloads
natifs ne sont pas publiés. Les projections de lecture restent dans `.tmp`,
hors Git et hors collection QMD. Les empreintes documentent la capture, pas un
contrôle cryptographique de chaque assertion des assistants.

Les pages publiques du dépôt contiennent cette distillation, des références et
des résultats déjà expurgés ; aucun export brut, credential, log OAuth, profil
de navigateur, kubeconfig ou raisonnement interne. Aucun sous-agent supplémentaire
n'est lancé pour l'ingestion. Les messages de coordination avec une autre session
coverage sont conservés comme contexte, sans importer implicitement son arbre.

## Trajectoire complète

1. **Rustfmt local/CI** : le même commit passait sous Rust 1.94.0 local et échouait
   sous stable 1.99.1 en CI. `rust-toolchain.toml` devient la source unique du pin
   1.99.1, l'action `.github/actions/setup-rust/action.yml` le consomme. Correction
   publiée `3d44ff6` ; fmt local et CI vérifiés. Le rustfmt plus récent révèle
   aussi une macro de tests à rendre compatible avec les virgules finales.
2. **Infrastructure offline** : un template Helm ne se parse pas comme YAML brut.
   Valider les rendus, sans exclure tout le chart et sans démarrer k6/Kind.
   Les défaillances IT et Clippy suivantes restent des frontières séparées.
3. **Découverte fonctionnelle IAM/OAuth** : pilote à quatre fonctionnalités et
   treize critères avec liens code/ADR/assertions, pas inventaire exhaustif IAM.
   Allure classe les labels fournis ; il ne découvre ni domaines ni exigences.
   Catalogue indépendant des tests pour rendre visibles les critères non testés.
   Les skills examinés sont des pistes, pas des installations ni une ratification.
4. **Oracles Q01/Q02** : caractérisation du comportement existant, sans changement
   produit : LOGIN sans email = 401/`authentication_failed`, erreur repository =
   500/`login_failed`, complétion = 200 et non 200/201. Deux unités et 17 IT PASS
   au jalon `a2c0f80`, pas de nouveau navigateur à ce jalon.
5. **Gates runtime** : six détecteurs lexicaux contredisaient des interfaces déjà
   ratifiées. Exceptions contextuelles fermées, pas d'exemption de fichier entier.
   L'architecte trouve deux contre-exemples : frontière Unicode et appel après
   `request.invoke.operation`. Le mémo de réconciliation reste **Proposed**.
   Les corrections de fixtures IT, dont SQS Telegraph, ne valent pas preuve E2E.
6. **Seam Ubuntu autonome** : producteurs CI Ubuntu 24.04 et bundles dev/debug
   non instrumentés, migration IAM et OAuthConnect explicites, quatre ELF vérifiés.
   Acquisition liée au workflow/repository/run/attempt/SHA ; recettes H et binaires
   T distingués. Le consumer empaquette sans Cargo. Packaging/ABI n'est pas E2E.
   Une garde indépendante `always()` ferme les probes par reçus, même si le
   `finally` du processus de packaging n'a pas pu s'exécuter.
7. **Chromium** : plusieurs essais rouges sont conservés comme diagnostics, pas
   des passes OAuth. `NO_USABLE_SANDBOX` ne prouve pas AppArmor. L'audit journald
   distingue absence d'outil, échec de capture, données vides, timestamps et
   identité PID ; zéro refus corrélé n'est pas une preuve d'absence de refus.
   Le launcher final utilise un profil seccomp dérivé de Moby, épinglé, avec
   delta de quatre syscalls. Ni mode privilégié, ajout de capacités, AppArmor
   global modifié, TLS ignoré, ni `--no-sandbox`.
8. **Preuve réelle** : attestation sandbox au diagnostic `43454504050`, puis
   campagne manuelle `43461703946`, 34 checks et contrat parent PASS. Décodage
   strict des titres de processus Linux réécrits par Chromium ; `/run/case` évite
   les chemins trop longs. Témoins kernel renderer, pas auto-déclaration seule.
9. **Automatisation et publication** : `7e5ec5e1` raccorde E2E après build et en
   parallèle des couvertures. Premier run `43470457080` bloqué avant E2E par
   `CARGO_CONFIG_OVERRIDE`. `741d0435` admet uniquement les deux alias Clippy
   déclaratifs connus dans le fichier repo, hashes conservés, pas les flags
   LLVM/wrappers/profiles/env/target ni alias shell/home.
10. **Réception finale** : `43473245852` déclenché sur push/master, H=T=packaging
    `741d04359a332de78802ef59cd8b2d1480297787`, SDK `93b890ff…` ; E2E et Pages PASS.
    Accueil/status/badge/Allure HTTP 200, 34 checks et un contrat parent séparé
    dans Allure (35 passes). 14/14 fichiers de preuve identiques et cleanup zéro
    ressource possédée active. Documentation finale publiée `7e7fc965`.

## Qualification, pas embellissement

La CI globale de `43473245852` est **rouge** sur l'IT Manifesto
`t6_post_get_components_contract_bytes_frozen` (job `132053039776`, test
`apparatus_p1_t6_http.rs:120` et panic Tokio `runtime/task/core.rs:422` ;
3 passes/1 échec dans ce binary). Cause non diagnostiquée dans cette session.
Ni « CI verte », ni couverture Sonar à 90 %, ni promotion d'ADR n'en sont déduits.

Les 16 unités du lot voisin `80104de4` sont incluses dans `manifesto-infra --lib`,
à distinguer des 22 binaries `--test` coverage90. Leur incorporation dans la source
CI ne donne pas un nouveau pourcentage de couverture.

Allure/Pages représente le dernier rapport publié et son SHA, pas le HEAD
instantané. Aucun timing de check, historique inter-runs ou métrique Sonar custom
n'est inventé. Le compteur 34/34 n'est ni une couverture produit exhaustive ni
une couverture de lignes Rust. Activation Pages explicitement autorisée par
l'utilisateur ; publication de projections fermées seulement.

## Sources durables recoupées

- `rust-toolchain.toml`, `.github/actions/setup-rust/action.yml`.
- `docs/discovery/iam-oauth/README.md`, `catalogue.json`.
- `docs/contracts/ci-runtime-gates-reconciliation-proposed.md`.
- `docs/contracts/k6-three-scenario-ubuntu-ci.md` (toujours Proposed).
- `docs/missions/20261113-k6-ubuntu-e2e-results.md`.
- `docs/missions/20261113-k6-auto-reporting-results.md`.
- `docs/local/audits/ci-cd/2026-11-13-k6-ubuntu-e2e.md` et
  `2026-11-13-k6-auto-reporting.md`.
- `ops/tests/k6/ci/README.md`, workflows CI/E2E et reporting.

L'ingestion ne rejoue aucun test ni commande historique et ne démarre aucun
runtime. Elle met à jour la connaissance, pas les verdicts de ces campagnes.
