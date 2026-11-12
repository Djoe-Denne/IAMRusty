# Réparation des jobs d'intégration CI

- Skill : ci-cd ; mode réparation ciblée, 12 novembre 2026.
- Cible : GitHub Actions `43278101994`, révision `4fa7cfa` ; travail repris sur
  `a2c0f80`. Aucun changement de workflow ni ajout d'instrumentation LLVM locale.
- Vérification : admission crypto6PASS ; IAM/Manifesto IT57PASS ; gates38PASS ;
  Telegraph outbox7PASS :108tests dans les binaires affectés,0ignoré/filtré.
  Ce rapport ne vaut pas PASS global, ni nouvelle décision d'architecture.

## P1 — Échecs vérifiés

- [x] **1. Reset de fixture Manifesto ignoré après refus de rétention**
  - Localisation : `services/Manifesto/tests/common.rs`, descriptor migrations.
  - Preuve : le down de la migration d'admission refuse intentionnellement ; le
    harness tolère les erreurs de down. Les suites resume/cleanup et outbox
    observent des lignes arrangées par des tests précédents (7 événements au
    lieu d'un dans `transaction_readiness_tests`).
  - Action appliquée : reset/reseed `fresh` exclusivement sur la base éphémère
    du harness, dans l'étape up dont l'erreur est bloquante. Pas de modification
    du down productif. Régression explicite table/outbox de la fixture suivante.
  - Validation : transaction9PASS (dont reset explicite), resume5PASS, cleanup4PASS.

- [x] **2. Tests des migrations historiques confondent initiale et chaîne courante**
  - Localisation : IAM `signing_epoch_writer`, Manifesto P1/P2/P3 migration tests.
  - Preuve : IAM attend 1 migration, en trouve 2 ; Manifesto attend 1, en trouve
    3. Le contrat runtime ajoute explicitement des migrations additives avec
    rétention. Un down complet n'est plus un rollback initial autorisé.
  - Action appliquée : initiale sélectionnée par nom ; roundtrip dans un schéma
    unique créé dans une transaction annulée. Manifesto compare les inventaires
    complets des tables/colonnes et vérifie séparément le refus de rétention du
    down courant. Aucun effacement de fence de production.
  - Validation : IAMsigning7PASS ; Manifesto P1/P2/P3 migrations8/12/12PASS.

- [x] **3. `kid` d'admission mal formé classé comme clé indisponible**
  - Localisation : `services/Manifesto/infra/src/apparatus_runtime/admission_proof.rs`.
  - Preuve : vecteur indépendant `header_kid_11` attend `RUNTIME_INVALID_PROOF`,
    obtient `RUNTIME_KEY_UNAVAILABLE`. Le contrat §13.5 impose `ara-v1-` suivi
    du thumbprint SHA-256 canonique, pas un identifiant arbitraire.
  - Correctif : validation de forme/canonicalité/32 octets avant lookup. Un `kid`
    bien formé non pinné reste `RUNTIME_KEY_UNAVAILABLE`, sans découverte réseau.
  - Validation : 6 tests crypto PASS, 0 ignoré/filtré, dont régression de cette
    distinction. Pas de changement des vecteurs indépendants pour verdir.

- [x] **4. Gates lexicales historiques et contrats runtime ratifiés contradictoires**
  - Localisation : gates Manifesto P1/P2/P3/P4 et corpus Lazaret scanné.
  - Preuve : interdictions globales de mots `lease`, `invoke`, `admission`,
    `Lazaret` alors que des interfaces ratifiées les ajoutent ; §11 exige aussi
    de préserver les gates. Il faut distinguer DTO métier et clients cluster/I/O.
  - Action : analyse d'architecture bornée, persistée dans
    `docs/contracts/ci-runtime-gates-reconciliation-proposed.md`. Racines,
    targets, tokens et exemptions préexistantes conservés ; seuls des
    identifiants exacts et fragments contextuels ratifiés sont reconnus sur
    des chemins exacts. Le reste de chaque ligne reste scanné. Ni gate désactivé
    ni nouvelle décision Accepted.
  - Validation : les six targets4/6/6/2/7/13PASS (38tests), dont dans chaque
    target un test de20cas positifs/négatifs lors de la première passe. Le corpus
    final est étendu à25cas après la revue ; ce n'est pas150tests distincts.
    Variantes de nom,
    autre chemin, vraie méthode, client cluster et ajout interdit sur la même
    ligne restent détectés. La revue a relevé puis fait corriger deux lacunes :
    suffixe Unicode traité comme séparateur d'identifiant et contexte trop large
    d'un champ DTO. Frontières désormais conservatrices et lectures admises
    fermées sur les expressions exactes ; nouveaux contre-exemples inclus.

- [x] **5. Telegraph : messages SQS d'un test précédent reçus après visibilité**
  - Localisation : `services/Telegraph/tests/outbox_runtime_tests.rs`.
  - Preuve : le run suivant43281256715, job131406446705, échoue sur
    `committed_claim_recovers_same_event_after_measured_expiry_with_fixed_rebuild_deadline`,
    avec `122 actual SQS envelope identity/version/payload mismatch`. Les
    témoins de recovery recevaient sans supprimer leurs enveloppes ; le témoin
    de cancellation conservait aussi volontairement un message après sa preuve.
    La fixture SQS est partagée et une réception ne vaut pas suppression.
    La [référence AWS DeleteMessage](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/APIReference/API_DeleteMessage.md)
    confirme la suppression explicite avec le reçu de la réception la plus
    récente ; cela ne promet pas une livraison exactly-once des queues standard.
  - Correctif : ack explicite par reçu seulement après les assertions exactes,
    y compris après avoir prouvé que le handler annulé n'ack pas. Pas de purge,
    filtre d'identité permissif, augmentation de délai ou modification du SDK.
    L'ack du témoin d'expiry reste dans sa deadline fixe d'observation.
  - Validation :7PASS/0ignoré/filtré en98.65s, run
    `it-10f04dc61485479eb529ab326200bad6`. Gates finales25cas :38PASS.
    Cette campagne ne suffit pas à prouver l'absence de toute flakiness CI.

## Vérifications auxiliaires et limites

La sélection fermée du lanceur natif permet les binaires affectés et conserve
les refus de noms arbitraires, mauvais services, casse, chemins et arguments.
Les tests AST/branches PowerShell passent sans démarrer Cargo/Docker. Ces tests
ont aussi détecté et fait corriger une comparaison de suite Manifesto
insensible à la casse.

Telegraph, Hive et Apparatus IT sont verts sur43278101994 ; IAM a fini avec
le seul binaire signing en échec. Telegraph redevient rouge sur43281256715 :
son succès initial n'était donc pas une preuve de stabilité. Pas de rerun à
l'identique pour masquer la flakiness.

Les logs CI et inventaires bruts restent sous `.tmp/ci-infra-static/`, non
publiés. Les campagnes natives utilisent le registre Docker append-only et le
cleanup du lanceur ; ni Kind, ni ressources préexistantes ne sont adoptés.

État local :5findings traités et validés. Publication et preuve CI complète
restent à obtenir. Les huit campagnes IT ont créé21fixtures et en ont retiré20,
0running/0inconnue/0souci cleanup. Le dernier conteneur, identifié au registre,
est arrêté `Exited(2)` et retenu en `ask` par le lanceur pour diagnostic :
`0fb66e94b62533893aaf0e7a65d15fc9d38138db71c15f96b5afff09f1773e39`.
Aucun retrait forcé ; trois conteneurs préexistants conservés. Registre :
`.tmp/docker-resources-20261112.log`.
Une première tentative transactionnelle n'a exécuté aucun test (imports
manquants dans la nouvelle régression, corrigés) et a créé0fixture ; elle ne
compte pas comme un PASS.
