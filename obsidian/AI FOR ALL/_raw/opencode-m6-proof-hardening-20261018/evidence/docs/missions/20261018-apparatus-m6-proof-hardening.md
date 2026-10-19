# Ordre de mission suivant — Apparatus m6 : preuve fiable et reproductible

**PROPOSITION — préparation seule.** Lire AGENTS.md, préparer/persister le plan et la policy de la nouvelle mission, présenter les tâches et attendre `/ensemble-mission start`. Aucun worker ni runtime avant ce GO. Ne pas réutiliser l'approbation de la mission close comme autorisation de démarrer celle-ci.

## Ancrage et lectures obligatoires

- Dépôt : `C:\Users\djden\source\repos\AIForAll` ; base de clôture `02dcd23f579a2e8ca3d3526046eb019bf433186e` ; code m6 `e66b736`. Revalider HEAD/porcelain, préserver tout changement ultérieur utilisateur.
- Pin rustycog immobile : `41a86dba0a24844f900783e19ef2405319d58b3f`. Ne modifier ni gitlink ni `.cargo/config.toml`.
- Ledger : `.cursor/handoffs/20261017-g17-e2e-chain-mission-prep.md` (section Clôture).
- Plans : `.cursor/qa-plans/20261017-operator-cli-real-binary-transport.md`, `20261017-m6-e2e-chain-kind-ready.md`.
- Revues : `.cursor/review-briefings/20261018-m6-e2e-chain-{correctness,tests,infra}.md` ; lire aussi les quatre briefings credentials du 17 pour les résidus antérieurs.
- Preuves : `.tmp/m6-evidence-20261018-pool14/MANIFEST.md`, `.tmp/m6-lead-recovery-20261018.md`, `.tmp/m6-post-run4-*.txt`, registres Docker des 17/18.
- Ingestion : `obsidian/AI FOR ALL/projects/aiforall/references/mission-g17-m6-20261018.md` ; inventaire des 19 sessions natives et des preuves sous `_raw/opencode-g17-m6-20261018/inventory.json` dans ce vault.

## Acquis — ne pas refaire

G-17 réel ORAS 1.2.2/Cosign 2.4.3 : t6 9/9, m1 1/1 ; secondaires filtrés 2+1+3 verts. m6 sur `kind-aiforall-local` : 1/1, 311,79 s. Correctness et tests GO_WITH_COMMENTS, infra PASS (13 contrôles, événements K8s expirés INCONCLUSIVE). Aucun test des quinze anciennes cibles Kind différées n'est réputé exécuté/compilé par le seul m6.

La chaîne utilise un vrai Pod et un vrai hop invoke ; elle n'est ni une simple sonde curl ni une certification de tout le mesh. Aucun changement de Réalité ADR 0412/0605 à déduire de ces acquis.

## Objectif borné

Fermer les commentaires du lot m6 par une tranche cohérente : **fixture non destructif testé, construction HTTP sans dérive de staging, canary probant, preuves par hop archivées**, puis UNE campagne finale Windows/IT et UNE exécution E2E Kind autorisée. Pas une refonte générale de l'E2E, du SDK ou du control-plane Ensemble.

## Registre des commentaires à traiter

- **C-1 MEDIUM** : m5 et `ops/deploy/apps/overlays/kind-demo-monolith/schedule-reference-kv.ps1` manquent toujours reqwest/rcgen dans leur mini-workspace. Le révert Dockerfile a réparé COPY, pas leur compilation. Ajouter les définitions racine exactes et une garde de cohérence des trois producteurs ; ne pas factoriser Rust/PowerShell sans nécessité démontrée. Le script gold path lui-même ne sera PAS exécuté.
- **C-2 MEDIUM / T-3 LOW** : résoudre kubectl par un probe client-only (`version --client`), sans dépendre du serveur/kubeconfig par défaut ; conserver le probe client-only de kind.
- **C-3 LOW** : config fixture devenue inutilisée, renommée `aiforall-local` mais subnet 192.168/16 divergent du canon 10.244/16. Préférer une référence alignée et explicitement non utilisée pour créer un cluster ; toute suppression exige vérification des références et décision tracée.
- **C-4 LOW** : actualiser les trois sites ops README/config qui disent encore « intouchable/ne pas retarget/Calico installé par fixture » ; documenter défaut local, override et fail-loud sans create/delete/install.
- **C-5 INFO** : normaliser le padding de l'override sans accepter de nom invalide.
- **C-6 INFO** : garde de périmètre aujourd'hui procédurale. Faire rejeter les noms protégés et les chemins/noms invalides AVANT tout appel externe ; figer les noms/alias et le comportement pour une cible non autorisée dans le contrat. Cette mission ne vise que `aiforall-local` ; full reste exclu.
- **T-1 MEDIUM / GAP-accept** : captures autonomes H1–H4 absentes. Ajouter des marqueurs/proofs non secrets réellement suffisants (digests, statut VALID, Scheduled, HTTP 403/200, hops, hostname, résultat canary). `--nocapture` seul ne produit pas les valeurs que le test n'imprime pas. Archiver des logs vérifiables sans argv/auth/config privée/clé/token.
- **T-2 LOW** : rendre readiness ET présence d'endpoints du canary fail-loud AVANT la sonde ; ajouter un contrôle positif de disponibilité et des négatifs qui distinguent cible absente/indisponible et refus réseau. Ne jamais attester NetworkPolicy sur « n'importe quelle erreur wget ».
- **M2/M10 acceptées** : tests discriminants pour absence de create/delete/install dans les branches d'erreur et pour SSA/field-manager sans force-conflicts, y compris préservation d'objets préexistants. Le plan QA choisit le niveau le plus petit adéquat ; ne pas provoquer une panne du cluster partagé pour tester une branche de fixture.
- **E-12 / registre G-1/G-2/G-3** : target effectivement sur E:, registre à la création, IDs réels et généalogie de remplacement ; création opaque = UNKNOWN+ask, jamais identification par seul nom. Le réseau du 20 septembre et les caches partagés ne deviennent pas possédés. Ne pas « réparer l'historique » par suppression/réécriture.

## Graphe proposé (à ajuster avant approbation)

### Commentaires hérités à garder distincts (pas de collision d'identifiants)

- **Revue G-17 correctness du 17** : N-1 = m5 corrigé statiquement mais jamais compilé par le lot (ne pas prétendre le contraire ; compilation possible comme gate de cohérence du staging, pas de run Kind m5). N-2 = log t6 run1 absent ; run2 suffit, ne pas fabriquer/rejouer une preuve historique.
- **Revue G-17 tests du 17** : T-1 = extraits zot cités sans capture brute autonome (accept historique ; la nouvelle capture vise les futurs runs). T-2 = destroys Cosign plus nombreux que creates, expliqué par suppression des prédécesseurs du fixture ; compléter la généalogie, pas « équilibrer » les logs.
- **Lot credentials antérieur, toujours hors correction ici** : S-1 LOW = APPARATUS_REGISTRY_PASSWORD/APPARATUS_VAULT_TOKEN hérités par certains enfants ; S-2 HARDENING = whoami/icacls résolus par PATH ; RP-A = thread lecteur temporairement détaché à timeout ; RP-B/N-1 = écriture stdin synchrone hors deadline pour le petit frame htpasswd ; N-2 = message générique de lecture imprécis ; sept tests cfg(unix) présents mais non exécutés sous Windows. Les citer au handoff, ne pas les mélanger avec C-1/T-1 de m6 ni ouvrir une campagne de transport CLI sans décision explicite.

Reprendre ensuite le graphe ci-dessous : le noyau de la prochaine mission est m6/proofs/fixtures, pas tous les résidus historiques du dépôt.

Scores **complexité / risque** indépendants ; pas des probabilités. Aucune sélection de modèle explicite : nouvelle policy mission native. Dernière grille utilisateur : light Luna none, standard GLM Flash max, strong Sol 6.1 xhigh ; **pas d'Opus**. Représenter la grille exacte dans le nouveau draft et la faire approuver, sans modifier les réglages globaux.

Photographie approuvée précédente (référence, pas autorisation de la nouvelle mission) : `.opencode/missions/team_mvcm6iko_0000_8ud2p5/model-policy.json`, revision 1, hash `a3c8460e40d50064570f880462b9922b59bfa6c8558fb74e63b5188abb3779ae`. Bandes `[0,34,67,101]` : les deux scores ≤33 → light ; l'un ≥67 → strong ; sinon standard. IDs/variants historiques : `opencode-go/gpt-6-luna` / `none`, `opencode-go/glm-5.3-flash` / `max`, `openai/gpt-6.1-sol` / `xhigh`. Vérifier disponibilité et approbation au moment de la préparation, sans override de routage.

1. **N0 — QA d'abord** (`qa-planner`, 52/55, sans runtime) : registre obligation→test incluant tous les C/T/M/E ci-dessus, critères d'échec probant, budget et commandes exactes depuis le Cargo.toml. Plan relu et commité avant tout writer.
2. **N1 — contrat ciblé** (`architecte`, 58/65, sans runtime) : port de commande testable/stratégie de garde de périmètre, causalité canary et schéma de preuves ; contrat proposé sous docs/contracts, sans code. Décider à partir du QA, pas redécouvrir le dépôt. Pas d'ADR rétroactive ni promotion ADR. Le lead fait approuver ce contrat avant implémentation si choix structurel nouveau.
3. **N2 — build/doc indépendant** (`test-engineer`, complexité retenue 70/risque45, worktree isolé) : propriétaire exclusif m5 staging, script staging, config fixture YAML de référence et trois sites docs ops ; tests de cohérence des producteurs dans un fichier nouveau dédié. Pas de run m5 Kind/gold path, pas de modification Dockerfile/logic applicative non prévue.
4. **N3 — fixture et oracles** (`test-engineer`, 72/65, autre worktree) : propriétaire exclusif `tests/fixtures/kind/mod.rs`, `tests/fixtures/chain.rs`, `tests/apparatus_m6_e2e_chain.rs` et tests ciblés nouveaux. C-2/C-5/C-6, T-1/T-2, M2/M10. Tests d'un adaptateur CLI sortant possibles ; vrais ORAS/Cosign et vrai Pod au hop final, jamais mockés pour la preuve réelle.
5. **N4 — intégration et gate** (lead, 55/60) : intégrer N2/N3, vérifier le diff et réserver UN slot Cargo. Windows uniquement, `CARGO_TARGET_DIR=E:\cargo-target\AIForAll` env-only et preuve du chemin effectif. Compilation/unités d'abord, IT réelles en fin de corrections, puis E2E. Pas de validation redondante d'un diff identique. Tout FAIL persisté avant correction, registre de tentatives continu.
6. **N5 — preuve finale réelle** (`test-engineer`, 60/65) : après intégration et préconditions QA, accord explicite pour le runtime de cette NOUVELLE mission ; registre opérationnel AVANT créations ; m6 avec captures instrumentées non secrètes et `--nocapture`, résultats/counts et empreintes archivés dans le dépôt canonique avant livraison. Pas de 15 anciennes suites Kind en bonus. Re-run supplémentaire seulement sur hypothèse bornée et GO du lead, jamais boucle.
7. **N6 — revues indépendantes** : correctness (42/60), tests (45/60), sécurité ciblée capture/secret et périmètre (40/70), infra-verifier lecture seule (40/50). Fresh, briefings persistés, findings traités avant clôture. Pas de reviewer obligé d'être d'accord avec le développeur.

N2/N3 peuvent travailler en parallèle sur fichiers disjoints, mais aucun Cargo parallèle. N0/N1 sont sans runtime. Le lead est la session racine, sans orchestrateur intermédiaire. N2/N3 font uniquement du code de test/fixture ; si le QA révèle un correctif de production nécessaire, isoler un lot implementer approuvé au lieu de le glisser au test-engineer.

## Critères de sortie / coupe de périmètre

- Registre C-1…C-6/T-1…T-3/M2/M10/E-12 fermé ou accept/defer EXPLICITE avec owner, impact, raison et condition de réouverture ; pas de disparition silencieuse de commentaire.
- Absence d'appels externes pour noms protégés/invalides, négatifs de préconditions CNI/cluster, SSA et client-only prouvés au niveau adéquat.
- Mini-workspaces cohérents avec les manifests hérités ; aucune prétention de run gold path/m5 si seuls leurs producteurs sont testés.
- Une chaîne m6 réelle verte, cibles et preuves identifiées, canary disponible avant preuve négative ; logs archivable et sans secrets ; pas de skip silencieux.
- Target E: prouvé, registre contemporané ou écart ROUGE explicite ; cleanup vérifié, ressources inconnues non adoptées.
- Budget : noyau ~75 %, buffer triage/review ~25 %. Couper d'abord toute extension mesh/CI et factorisation ; jamais canary, garde de périmètre, confidentialité des captures ou traçabilité. Deux tentatives techniques sans progrès vérifiable : escalade selon AGENTS, pas de quatrième worker identique.

## Discipline tirée de cette session

- Écritures via patch/outil dédié, pas de here-string PowerShell géant ; squelette tôt, édition incrémentale.
- Un worker idle n'implique PAS un test fini : exiger notification/exit code/résultat libtest, ne jamais relancer tant que le processus ou le bail vit. Éviter les commandes d'attente non bornées.
- Avant shutdown d'un writer : livraison/état Git/proofs ; fermer ou bloquer sa tâche avant toute réadmission pour éviter les writers doubles. Ne pas publier une seconde tâche qui écrit le même plan.
- Leaf sans délégation ; lead seul tient ledger, modèles imposés par policy. Continuer le même propriétaire tant que sa session est disponible ; reviewers fresh.
- Faire porter les garde-fous dans le prompt initial et le contrat, pas seulement dans un message qui peut attendre derrière un call bloqué.

## Hors mission

Lab OAuth 0412/full, changement de Réalité ADR, gold path 0605 complet, migration/factorisation SDK, gros renforcement mesh, réparation du harness OpenCode ou globale des hooks, les résidus credentials S-1/S-2/RP-A/RP-B/N-1/N-2 et sept unités cfg(unix), m2/t12/t2/t7 périmés. Les mentionner comme résidus hérités, pas les implémenter sans nouveau GO. Aucun Docker/Kind/WSL ne démarre pour préparer ce plan.
