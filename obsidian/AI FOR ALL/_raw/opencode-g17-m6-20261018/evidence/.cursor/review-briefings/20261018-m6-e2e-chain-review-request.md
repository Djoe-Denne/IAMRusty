# Demande de revue indépendante — P6 / chaîne m6

- Dépôt canonique : `C:\Users\djden\source\repos\AIForAll`.
- Diff : `43c3804..e66b736` ; quatre fichiers (fixture Kind, config fixture, chain.rs, Dockerfile.http).
- Plan QA : `.cursor/qa-plans/20261017-m6-e2e-chain-kind-ready.md` ; E-1…E-12 et ajouts E-13/E-14 au §9bis.
- Continuité obligatoire : `.cursor/handoffs/20261017-g17-e2e-chain-mission-prep.md`. Les conclusions historiques sont des hypothèses à rejuger, pas les conclusions attendues de cette revue.
- Preuves : `.tmp/m6-lead-recovery-20261018.md` donne les chemins absolus des logs du worktree préservé, les observations datées et leurs limites. Aucun nouveau run de test demandé.
- Résultat libtest observé : `m6_e2e_0002_0008_chain`, 1 passed / 0 failed / 0 ignored / 0 filtered out, 311.79 s.
- GO utilisateur : Kind `aiforall-local` seulement ; pas full/0412 ni clusters protégés ; révert Dockerfile autorisé dans le même worktree. Pas de promotion ADR demandée.

## P6a correctness

Contrôler indépendamment le comportement observable du diff, la résolution de cluster/kubeconfig et les noms dérivés, l'absence de remédiation destructive, les préconditions CNI, les limites d'apply sur le cluster partagé, la restauration du contexte de build et les assertions conservées. Distinguer régression introduite, défaut préexistant et écart documentaire. Rendre des findings localisés et un verdict dans `20261018-m6-e2e-chain-correctness.md`.

## P6b tests (après décision sur P6a)

Auditer le registre E-x contre les preuves réellement disponibles, avec mutation mindset : un run vert ne suffit pas à déclarer toute obligation contractuelle couverte. Distinguer assertions exécutées, captures autonomes, présence statique et scénarios non exécutés. Rendre `20261018-m6-e2e-chain-tests.md`.

## Vérification infra indépendante

Lecture seule du contexte `kind-aiforall-local` uniquement : état nœud/CNI, CRD, namespaces, Pod pin/digest/labels, CR VALID, canary Ready/Service/endpoints, NetworkPolicy, événements bornés et cleanup Docker/process. Ne pas lire les Secrets ni lancer exec, test, apply, build, création/suppression. Rendre PASS/FAIL/INCONCLUSIVE avec preuves datées ; le lead persistera ce rapport.

## Contraintes communes

Pas de Cargo, de second m6, de runtime nouveau, de configuration globale, de gitlink/config Cargo modifiés. Écritures via outil dédié, jamais par here-string shell. Reviewers indépendants fresh, pas d'implémentation corrective. Le lead seul met à jour INDEX/ledger/plan.
