# Mission1 k6 — plan borné proposé

## Objectif et statut

Installer un petit socle fonctionnel k6 et un smoke réel sous Linux. Préparation uniquement, policy non approuvée, aucun worker ou runtime lancé. Le titre natif « End-to-end test using K6 » ne vaut pas extension aux missions OAuth et droits.

La cible de qualification est Linux natif. L'utilisateur autorise WSL ou Docker pour les essais locaux, GitHub Actions pour Linux natif et propose un démarrage Linux si nécessaire. Ces choix ne permettent ni runtime privilégié, affaiblissement TLS/sandbox, modification produit, ni nouveau déployeur. Aucun dispatch GitHub, commit ou push autorisé implicitement.

## Sources et preuves réutilisées

- AGENTS.md et règles de handoff/harness ; ADR0200–0202 Accepted.
- docs/adr/0504-validation-fonctionnelle-linux-k6.md : Proposed / Unimplemented.
- docs/contracts/k6-functional-linux.md : Proposed / Unimplemented, V1 à ratifier avant code.
- docs/missions/20261029-k6-01-socle-smoke.md et recherche/enchaînement du même jour.
- docs/contracts/20261029-auth-security-qualified-state.md : qualification locale0412 uniquement, pas preuve k6. 17 IT Windows et navigateur Chrome149 historiques, laboratoire arrêté en fin de campagne selon cette preuve.
- docs/reviews/20261029-worktree-consolidation.md:77–121 : dernière mission archivée, 23 worktrees de ce lot retirés, autres worktrees préservés, 24 profils intégrés.
- services/IAMRusty/http/src/lib.rs:51–65 et public_routes.rs:25–31 ; rustycog/rustycog-http/src/lib.rs:39–40 : health public, corps exact OK.
- ops/deploy/local_full_target.py : LocalClusterTargetAnchor.check appelle _uid_check, validations locales avant/après UID ; approved_root/private_file ont des branches POSIX. Compatibilité opérationnelle Linux non prouvée ; ne pas copier un bail Windows.
- .github/workflows/ci.yml : unique workflow observé, builds/tests Linux natifs et contrôle infra statique, pas de job k6 ou de déploiement Kind identifié. Ne pas lancer ce workflow pour préparer.

## Graphe et ownership proposés (aucune tâche admise)

1. **Figer le contrat et le raccordement Linux** — lead, avec architecte si arbitrage structurel. Ratification explicite de la V1 avant source ; déterminer voie conforme des gardes et cible accessible. Pas de modification globale des gardes ou nouvelle grammaire de bail sans arbitrage. Livrable : contrat de travail ratifié, pins/provenance candidats, périmètre CI exact. L'identité de la cible ne se déduit jamais de son URL.
2. **Livrer le socle, le smoke et la CI minimale** — un implementer, worktree isolé ; dépend de 1. Ownership ops/tests/k6/**, .github/workflows/k6-functional-smoke.yml, documentation de ces éléments. Un seul auteur car CLI, résumé, cas et workflow doivent partager un contrat cohérent. Consommateurs non publiés avant intégration. Le lead peut en parallèle vérifier les sources officielles de provenance et la documentation CI, sans mutation des mêmes fichiers ni démarrage runtime.
3. **Vérifier les risques de cible et de confidentialité** — revue indépendante fresh bornée, après arrêt/intégration de l'auteur. Security-reviewer utile pour allowlist, environnement, processus possédés, artefacts et permissions CI ; pas de quatre revues systématiques. Findings bloquants corrigés dans la même session auteur ; suggestions non bloquantes séparées. Infra-verifier seulement si modification runtime/infra significative effectivement admise, pas autorisation d'étendre l'infra.
4. **Qualifier Linux et clôturer** — lead, après intégration et résolution des findings bloquants. Contrôles statiques, selftests, Chromium puis smoke réel sur cible autorisée ; inventaires/cleanup sur succès et échec. Pas de compilation Cargo prévue pour des changements exclusivement outillage/docs. Une preuve CI nécessite un vrai run consultable ; écrire le YAML n'en est pas une. Si la cible manque, poursuivre qualification indépendante, conserver HTTP INCONCLUSIVE et proposer la reprise Linux sans inventer de PASS.

Pas de classification ou scores demandés, pas de modèle forcé. Routage par catalogue approuvé, session/modèle de tâche conservés. Aucun démarrage avant start humain. Pas de consommateurs de branches isolées publiés avant intégration réelle.

## Livrables et interface proposés

- ops/tests/k6/README.md, config/ (exemple sans secret et pins/provenance/checksums), lib/ seulement si utile.
- scenarios/smoke/iam-health.js : un GET /iam/health, HTTP200, corps exact OK, 1VU/1itération.
- scenarios/smoke/browser-prerequisite.js : Chromium réel sur about:blank, sans UI ni OAuth.
- run-linux.py : entrée mince, sous-commandes validate, run, browser, selftest. validate sans appel réseau ni réveil runtime ; run/selftest sous garde validée, browser sans cible applicative. Appels k6 internes explicites avec usage reporting désactivé. Aucun exécuteur arbitraire ou API admin.
- tests/test_*.py : validations locales, complétude, sorties, refus avant accès, orchestration des négatifs. Pas de serveur HTTP factice ajouté.
- .github/workflows/k6-functional-smoke.yml : jobs Linux qualification outillage/Chromium et HTTP réel séparés. Pas de service cible arbitraire fourni par une PR, secrets aux PR non fiables ou auto-déploiement. Absence de cible explicitement non qualifiée, pas skip transformé en succès global.

Les commandes exactes proposées et critères sont dans model-policy.json.verification. Tout changement conséquent de cette interface de preuve impose correction du brouillon/présentation avant admission correspondante, pas expansion silencieuse du harness.

## Frontières et inconnues

ADR0504/contrat non ratifiés à ce stade. Runner local WSL préféré mais non démarré, distribution non inspectée. Pin Chromium exact à fixer avec provenance vérifiable, pas inféré du Chrome149 historique. Cible HTTP Linux/CI et bail à désigner/vérifier. Le runner GitHub hébergé ne peut pas joindre le loopback du laboratoire Windows ; aucune solution par tunnel ou nouveau déployeur n'est incluse. Ajouter un déploiement éphémère CI ou un nouveau contrat de propriété nécessite arbitrage avant code.

## Annexe technique

HEAD observé c3c65965c2ffb2c4b6b01d5b2efce5a5edb9605d ; SDK02e5db23aac6016aab062c3078696d5f7fd464ef ; index vide à l'observation. WIP étrangers Obsidian/SDK et missions locales préservés. Inventaire Git : 69 entrées racine comprise, répertoires existants, un worktree verrouillé ; aucune suppression. Revalider avant admission et intégration.

Sources officielles consultées : https://github.com/grafana/k6/releases/tag/v2.3.0 ; https://grafana.com/docs/k6/v2.3.x/using-k6/test-lifecycle/ ; https://grafana.com/docs/k6/v2.3.x/using-k6-browser/running-browser-tests/ ; https://grafana.com/docs/k6/latest/using-k6/scenarios/executors/per-vu-iterations/ ; https://grafana.com/docs/k6/latest/using-k6/thresholds/ ; https://grafana.com/docs/k6/latest/results-output/end-of-test/custom-summary/ . Les pages latest sont mobiles ; confronter les API utilisées à 2.3.x et aux binaires épinglés.
