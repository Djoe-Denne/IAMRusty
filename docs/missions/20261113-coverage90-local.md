# Tests locaux vers 90 % — corrections et validations locales

## Publication autorisée après validation

Après nettoyage et présentation des résultats, l'utilisateur a autorisé le commit
et le push de ce lot sur la branche principale effective **master**. Cette nouvelle
autorisation remplace la restriction initiale « non committé » pour ces livrables
seulement. Aucun changement de workflow ni déclenchement CI manuel dans ce lot.
La session de coordination « Échec de cargo fmt sur la CI » doit être prévenue
après vérification du push pour raccorder ses lanceurs à cette campagne.

### Points de raccordement

- Le lanceur `ops/tests/it-native.ps1` accepte les trois paires exactes
  `hive/coverage90_invitation_transitions`, `hive/coverage90_member_roles`,
  `manifesto/coverage90_member_persistence`. Régression de sélection/refus étendue.
- Les129 nouveaux cas purs sont dans les cibles `tests/coverage90_*.rs` des crates
  IAM/Hive/Manifesto/Telegraph/application/domain/http/infra et apparatus-contracts.
  Ce sont des binaries Cargo `--test` : **`--lib --bins` seul ne les exécute pas**.
  Le raccordement distant doit explicitement collecter ces cibles et leurs profils.
- Réutiliser les fixtures PG/OpenFGA existantes, Windows natif pour le local,
  queues opt-in inchangées ; E2E ne doit pas être confondu avec cette couverture.
- L'exclusion harness TEST-ONLY est le seul ajustement du périmètre coverage.
- La métrique85,73 % reste une estimation hybride locale ; le raccordement CI
  devra produire la mesure distante autoritative, sans l'annoncer anticipativement.

## État après corrections autorisées

L'utilisateur a ensuite autorisé la correction locale des quatre défauts découverts,
toujours **sans commit, push ni changement CI**. Correctifs intégrés : annulation Hive
scopée organisation et404 générique ; pagination/count exacts ; remplacement des
rôles transactionnel avec outbox ; mapper Telegraph déléguant au canon du domaine.
Contrat : `docs/contracts/coverage90-hive-corrections.md`, applicable au GO local.

- Les **146 tests ajoutés** ont des preuves vertes après corrections, avec réutilisation
  des résultats des cibles non affectées :129 unitaires et17 IT (11Hive,6Manifesto).
  Ce n'est pas un rejeu simultané complet de tous les services sur le dernier HEAD.
- Régression **Hive complète :78 PASS,0FAIL,0ignored**,15binaries ; inclut les11nouveaux
  tests Hive. Tests libs/probes Hive :22PASS supplémentaires. Ne pas additionner
  ces nombres comme des tests tous distincts des campagnes ciblées.
- Preuves réelles : rollback tardif `BadEvent` avec snapshots membre/liens/outbox ;
  deux writers PostgreSQL réellement bloqués/chevauchés sur parent initialement sans
  grants ; résultat final non cumulatif et événements pending exacts ; issuer cible,
  rôles vides/dupliqués, PATCH répété,404 générique et pagination tous statuts.
- Reviews **correctness et sécurité PASS statiques**, delta QA conforme. Elles ne
  remplacent pas les résultats exécutés ci-dessus.
- Limites non bloquantes : retrait concurrent contre remplacement non rejoué ;
  oracle du tri secondaire non démontré mutation-fort. Pas de garantie nouvelle
  de snapshot liste/count, d'autorisation acteur atomique ni d'ordre/exactly-onceFGA.

### Couverture après modification des sources

Les numéros de lignes des fichiers modifiés ne sont **pas fusionnés avec leur
ancienne version**. Export actuel séparé :5binaries recompilés après correctifs,
1403profils, export LLVM sans warning. Il remplace intégralement les10fichiers
modifiés qui avaient des lignes mesurées ; les autres sources réutilisent la baseline
identique et les exécutions locales.

Cette mesure hybride donne **85,732 %** (41710lignes au dénominateur LLVM révisé),
encore1780lignes pour90 % selon ce modèle. Ce n'est **ni une nouvelle mesure Sonar,
ni un rejeu local intégral**. L'objectif90 % n'est pas atteint ; aucune exclusion
supplémentaire n'est utilisée pour le masquer. Rapport calculé :
`E:/cargo-target/AIForAll/cv90-evidence-20261113/cv90-current-export/comparison.json`.

### Ressources après les campagnes finales

Trois IT initiales :6créés/6retirés ; cibles Hive corrigées :4/4 ; suite Hive complète :
25/25 (pic3). **35 conteneurs de fixtures créés et35retirés,0possédé running**, aucun
nouveau conteneur non attribué ni issue cleanup dans ces six campagnes complètes.
Les quatre ressources préexistantes (3Kind+SMTP) sont préservées ; baux recovery et
IT libérés. Registre : `.tmp/docker-resources-20261113.log`.

Les sections suivantes conservent la chronologie et les preuves avant correction.

## Mandat et portée

Travail **local, non committé**, sans push, déclenchement CI ni modification
de workflow. L'autre mission CI/E2E est préservée. Le code de production était
inchangé lors de la première phase de tests ; les quatre correctifs autorisés
ci-dessus sont l'unique extension. La maintenance runtime signée et les parcours HTTP trusted/mTLS
difficiles restent différés.

Seule exclusion ajoutée : `crates/apparatus-contracts/src/harness.rs`, explicitement
TEST-ONLY. Aucun DTO, commande, entité ou code de sécurité exclu ; CPD inchangé.

## Livrables et résultats réellement exécutés

25 nouveaux fichiers `coverage90_*` :
- IAM : validation/mapping des commandes, contrats HTTP et redaction, DTO publics,
  transitions compte/email/tokens.
- Hive/Manifesto : transitions métier, permissions, invitations, validation des
  commandes et des vocabulaires ; IT HTTP/PG/OpenFGA de persistance et refus.
- Telegraph/apparatus-contracts : communications, erreurs, parsing/validation,
  capacités, identifiants, formats wire et bornes.

Avant correctifs production, après corrections d'attentes de caractérisation et review :
**141 tests : 137 PASS, 4 FAIL**, aucun ignoré dans les campagnes recensées.
Unitaires : **128 PASS / 1 FAIL** ; IT : **9 PASS / 3 FAIL**, dont Manifesto **6/6**.
Ce n'est pas une campagne verte. Tous les nouveaux binaries compilent ; les
contrôles Clippy ciblés pedantic/nursery et le formatage des 25 fichiers passent,
sans diagnostic restant attaché aux nouveaux fichiers. Les avertissements
préexistants hors fichiers ne sont pas présentés comme résolus.

Les reviews indépendantes valident la conception des tests et identifient les
défauts exposés ; ce verdict ne constitue pas un PASS de production.

## Échecs conservés

1. **Hive, isolation des invitations — confirmé par IT.** Une autorisation Write
   sur une organisation permet l'annulation d'une invitation étrangère : HTTP200
   et modification de la ligne étrangère. Test
   `write_on_another_organization_cannot_cancel_a_foreign_invitation`.
2. **Hive, pagination — confirmé par IT.** Cinq membres avec page_size2 :
   `total_items` vaut2 au lieu de5. Test
   `member_page_totals_and_cursors_describe_the_whole_organization_not_the_current_slice`.
3. **Hive, remplacement des grants — défaut confirmé après relecture.** Après
   demande de remplacement read→write, la DB contient read+write ; le test attend
   write seulement. Le service domaine insère les nouveaux liens sans retirer
   les anciens ; l'événement publié et son traducteur Sentinel Sync traitent les
   rôles comme un ensemble final de remplacement. La divergence DB est exécutée ;
   l'incohérence attendue avec FGA après consommation est une conclusion statique,
   pas une mesure E2E de ce run. Correctif transactionnel à arbitrer séparément.
4. **Telegraph, classification d'erreurs — confirmé, latent.**
   `publish_errors_keep_typed_status_and_message` échoue infrastructure vs business.
   Le mapper infra diverge du `From<&DomainError>` déclaré par le domaine. La revue
   ne trouve pas d'appel production actuel à cette direction du mapper : pas de
   poison-retry actif démontré. L'assertion reste intacte, aucun correctif prod.

## Couverture : mesure bornée, pas 90 %

Baseline existante : 41827 lignes mesurées, 7219 non couvertes. Le LCOV de la
CI réussie43414594622 est **réutilisé**, aucun nouveau run distant déclenché.
Réconciliation complète des chemins/lignes avec le snapshot Sonar.

L'exclusion retire248 lignes, dont166 déjà couvertes : **82,835 %** avant nouveaux
tests. Instrumentation locale Windows/LLVM correspondant à Rust1.99.1 :
25 exécutables,4113 profils fusionnés, export sans avertissement.
**1206 lignes baseline auparavant non couvertes sont exécutées** ; union baseline
+ tests locaux + exclusion : **85,736 % projetés**, encore **1774 lignes** à couvrir
pour90 % à ce périmètre.

Cette union inclut les tests rouges exécutés. Elle n'est ni un rejeu local complet,
ni une nouvelle analyse Sonar, ni une mesure prouvant la correction fonctionnelle.
Les nouveaux scénarios IT peuvent apporter une forte valeur sans beaucoup de
lignes nouvelles si leurs chemins étaient déjà parcourus dans la baseline.

## Commandes et preuves

- `.tmp/cv90-units.ps1` :11 commandes Cargo ciblées séquentielles, --locked,
  --no-fail-fast, --message-format=json ; modes test puis Clippy.
- `.tmp/cv90-unit-review.ps1` : retests des cinq binaries A/B affectés.
- Tests/Clippy ciblés finaux : apparatus-contracts/coverage90_runtime_boundaries,
  telegraph-domain/coverage90_error_categories.
- `cargo test --locked --no-run ...` et Clippy sur hive-service (deux binaries)
  et manifesto-service/coverage90_member_persistence : compilation/contrôles,
  pas preuve d'exécution à eux seuls.
- `.tmp/cv90-it.ps1 -RunLabel cv90-it-run-2` utilise exclusivement le lanceur
  `ops/tests/it-native.ps1` et ses sélections exactes ajoutées/testées localement.
- Instrumentation `RUSTFLAGS=-C instrument-coverage`, LLVM_PROFILE_FILE explicite,
  profils/logs sous `E:/cargo-target/AIForAll/cv90-evidence-20261113/`.
- `.tmp/cv90-export-units.cjs --with-it`, puis `.tmp/cv90-compare.cjs` sur
  `cv90-combined-export/units.lcov`. Manifestes et inventaires conservés.

## Environnement et cleanup

La première tentative IT a été interrompue par C:saturé ; ce n'est pas un test
ROUGE. Les profils partiels restent archivés et sont exclus de la mesure finale.
Déplacement vers E: autorisé par l'utilisateur, inventaires préservés, jonctions
aux anciens chemins. Aucun cache supprimé pour contourner le blocage.

Docker Desktop était ensuite bloqué côté backend. Diagnostic puis arrêt gracieux
échoué et arrêt forcé ciblé ont chacun reçu un GO distinct. Le daemon est rétabli ;
les trois nœuds Kind préexistants ont redémarré par leur policy et sont restitués
au runtime utilisateur/autre agent. Leur état container ne prouve pas à lui seul
la santé de tous les pods. Aucun conteneur/volume supprimé par la recovery.

La campagne IT finale a créé **6 conteneurs, retiré6, aucun possédé running**, aucune
issue cleanup ni nouveau conteneur non attribué. Inventaire final = les4IDs
préexistants (3Kindrunning,SMTPexited). Le registre contemporain se parse entièrement
en JSONL ; les anciens événements restent append-only. Les baux recovery et IT
sont libérés ; le runtime étranger n'est pas arrêté en fin de lot.

## Limites initiales et suite

- Les quatre échecs initiaux ont été conservés comme preuves avant correction,
  puis corrigés après GO distinct et validés ; voir l'état final ci-dessus.
- 90 % non atteints ; ne pas élargir les exclusions pour masquer la logique métier.
- Après GO explicite de nettoyage, les6worktrees créés par la mission ont été
  retirés. Archives récupérables :
  `E:/cargo-target/AIForAll/cv90-evidence-20261113/cv90-worktree-archives/` :
  38fichiers auteurs et38versions canoniques finales, SHA256 vérifiés. Uniquement
  le checkout master reste présent ; aucune branche temporaire n'avait été créée.
- Au contrôle de clôture locale précédant l'autorisation de publication, les25fichiers
  de tests étaient non suivis, les13fichiers source modifiés et l'index Git vide.
  HEAD6bdd8a50 avait évolué par l'autre mission ; aucun commit/push de ce lot
  n'avait encore été effectué. La publication autorisée est décrite en tête du rapport. Sous-module RustyCog
  inchangé à93b890ff7b821e9cb2e7d0aeb81957d098778a2f.
- Les35IDs de fixtures sont absents de l'inventaire final ; les4IDs préexistants
  sont conservés.196lignes du registre quotidien parsées,0ligne invalide. Aucun
  arrêt global Desktop/WSL en fin de lot : le cluster étranger est actif.
- Ledger : `.cursor/handoffs/20261113-coverage90-local.md` ; reviews persistées
  sous `.cursor/review-briefings/20261113-coverage90-*`.
