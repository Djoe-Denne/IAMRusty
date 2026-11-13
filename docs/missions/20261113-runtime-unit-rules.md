# Unités runtime : règles réelles, sans confiance fabriquée

## Périmètre

Mandat : compléter les tests unitaires de maintenance et de preuve, sans substituer
un faux certificat/vérificateur permissif à la frontière de confiance. Envoy et la
preuve mTLS restent dans les E2E ; aucun terminateur TLS n'est ajouté pour ces tests.

Les migrations étaient **déjà exclues** du coverage par `**/migration/src/**` dans
`sonar-project.properties`. Aucune règle d'exclusion n'est ajoutée ou élargie.
Les nouveaux modules sont sous des répertoires `tests/`, déjà exclus, pour ne pas
gonfler le dénominateur avec le code des tests eux-mêmes.

## Implémentation

- `services/Manifesto/infra/src/apparatus_runtime/runtime_control/maintenance/tests/mod.rs` :
  **9 tests**. Commande/version/identifiants concordants et discordants ; accusé dans
  le passé, à l'égalité et1msdans le futur ; représentation du plan comparée à un
  JSON littéral indépendant (identités/digests distincts, compteur2^53+1en chaîne) ;
  littéraux d'action fermés ; gardes de configuration désactivée, plan incompatible,
  canal absent. `Disconnected` sert uniquement aux refus avant toute opération SQL.
- `services/Manifesto/infra/src/apparatus_runtime/admission_proof/tests/mod.rs` :
  **7 tests**. Vraie vérification RSA sur corpus Node/JCS préexistant ; révocation
  ciblée/idempotente ; révocation bloquée par deux capacités réellement vérifiées,
  réveillée après leur libération ; allowlists fail-closed, UUIDnil/exposant incorrect,
  repositories exacts, segments stricts et frontière16384/16385octets discriminante.
- Dans les deux fichiers parents : seulement `#[cfg(test)] mod tests;`.
  Aucun changement de logique publique, port, dépendance, fixture ou politique TLS.

Pas de clé privée sauvegardée ni de signer ajouté. Les positifs passent par le
vrai `PinnedRuntimeAdmissionProofVerifier`; les grandes frames sont des sondes
négatives, jamais présentées comme des preuves authentifiées. Aucun canal retournant
`Ok` ou faux vérificateur retournant `true`, aucun SQL simulé.

## Preuves locales

Windows natif, Rust1.99.1,24jobs configurés, un Cargo à la fois. Exécution séquentielle :

```text
cargo test --locked -p manifesto-infra --lib apparatus_runtime::runtime_control::maintenance::tests
cargo test --locked -p manifesto-infra --lib apparatus_runtime::admission_proof::tests
cargo clippy --locked -p manifesto-infra --lib --tests -- -W clippy::pedantic -W clippy::nursery
cargo test --locked -p manifesto-service --test apparatus_runtime_admission
```

Résultats : **9/9 et7/7nouveaux tests verts**, puis **6/6régressions cryptographiques
existantes vertes**. Zéro échec et zéro ignoré ; les commandes ciblées filtrent
respectivement13et15autres tests de la bibliothèque. Ce n'est pas un run complet
de toutes les cibles de l'espace de travail.

Première commande Clippy réussie avec un seul diagnostic nouveau `doc_markdown` ;
backticks ajoutés suivant le précédent du skill `aiforall-sonar-policy`, section
« Campagne2026-10-08—règles traitées ». Registre partagé consulté, aucune nouvelle
décision ni exemption créée. Contrôles finaux **check--tests,Clippy,fmtrust4fichiers,
diff--checkPASS** ;aucundiagnosticattachéauxnouveauxmodules. Les144émissionsde
warnings hors nouveaux fichiers ne sont pas présentées comme144issues uniques ni
comme résolues par ce lot.

Instrumentation explicite `RUSTFLAGS=-C instrument-coverage` et `LLVM_PROFILE_FILE`
sous `E:/cargo-target/AIForAll/runtime-unit-rules-20261113/run-1/` ; aucun nouveau
pourcentage global Sonar ni gain de couverture extrapolé de ces tests.

## Revue et limites

Review indépendante : **PASS statique borné**, aucun finding bloquant ni test
auto-réalisateur identifié ; l'exécution ci-dessus établit le résultat comportemental.
Limites conservées :

- gardes du constructeur `with_verified_channel` non exercées (suggestionLOW) :
  son invocation exige un objet canal ; pas de fabrication de canal pour gagner
  cette couverture ;
- cohérence des accusés après le vrai canal transférée à la frontière IT/E2E ;
- aucune preuve mTLS/chaîne CA, provenance opérateur/D, transaction PostgreSQL,
  persistance ou comportement single-writer Adm-A apportée par ces unités.

Les16tests sont complémentaires aux6familles cryptographiques `g7_*` existantes,
pas une duplication de la chaîne E2E. La vraie vérification de signature ne prouve
pas, à elle seule, l'authentification du transport.

## Coordination et ressources

Slot Cargo coordonné avec la session CI ; attente tant qu'il était occupé, puis
validation séquentielle. Bail désormais libéré et session CI prévenue. Aucun Docker/Kind/WSL démarré, aucune CA installée dans un
profil utilisateur ou runner, aucune ressource Docker créée par ce lot. Le travail
étranger CI/E2E et son audit sont préservés. Aucun workflow ni lanceur CI modifié.

Après GO utilisateur «Archiver et publier», les deux worktrees auteurs ont été
retirés. Leurs4fichiers et les4versions canoniques finales sont archivés avec SHA256
vérifiés sous `E:/cargo-target/AIForAll/runtime-unit-rules-20261113/worktree-archives/`.
Les originaux `tests.rs` sont conservés dans l'archive ; les versions finales
`tests/mod.rs` incluent le déplacement et le chemin de fixture adapté.

Publication autorisée : six fichiers de ce lot seulement (4Rust,planQA et ce rapport),
sur **master**, branche principale effective. Aucun workflow ni changement étranger
embarqué ; notification de la session CI après push et référence distante vérifiés.

Ledger : `.cursor/handoffs/20261113-runtime-unit-rules.md`. QA :
`.cursor/qa-plans/20261113-runtime-unit-rules.md`. Revue :
`.cursor/review-briefings/20261113-tests-runtime-unit-rules-186a63c0.md`.
