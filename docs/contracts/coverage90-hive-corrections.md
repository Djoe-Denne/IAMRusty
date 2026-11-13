# Coverage90 — correctifs Hive bornés

- Date : 2026-11-13 ; base inspectée : `3fe53b43bf4bad6d63d734f2934521710a0b2c30`, SDK `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`.
- **Statut du contrat : applicable au correctif LOCAL autorisé par le GO utilisateur du 2026-11-13 ; non implémenté lors de cette rédaction initiale.** Ni ADR `Accepted`, ni promotion globale de Réalité. Photographie après intégration : correctifs locaux et 11 IT Hive ciblées verts, suite Hive 78 tests verts ; limites retrait concurrent et force de l'oracle de tri documentées dans [le rapport](../missions/20261113-coverage90-local.md). Aucun commit/push/CI.
- Canon secondaire d'exécution de [0402](../adr/0402-hive-organisations.md), [0103](../adr/0103-ports-adapters-command-factory.md), [0301](../adr/0301-outbox-transactionnel-rustycog.md) et [0314](../adr/0314-principal-authority-through-openfga.md), sans modifier leurs Statut/Réalité. Tests : ADR [0200](../adr/0200-it-infra-reelle-rustycog-testing.md), [0201](../adr/0201-mocks-http-sortant-seulement.md), [0202](../adr/0202-transport-opt-in-queues-desactivees.md).
- Mandat/preuves : [ledger](../../.cursor/handoffs/20261113-coverage90-local.md), [QA](../../.cursor/qa-plans/20261113-coverage90-local.md), [revue C/D](../../.cursor/review-briefings/20261113-coverage90-tests-cd-result.md), [arbitrage remplacement](../../.cursor/review-briefings/20261113-coverage90-hive-replace-result.md).

## 1. Décision et périmètre

Corriger les trois défauts confirmés par les IT réelles : annulation interorganisation, total/pagination des membres, remplacement des rôles. L'intention est **remplacement de l'ensemble des grants**, non ajout cumulatif, avec liens et intention d'événement dans **une transaction writer Hive**. L'UoW existante porte déjà cette responsabilité : aucune nouvelle frontière, ADR ou ratification requise. Le verrou de ligne est un détail de cette transaction, pas un protocole de lease/CAS.

Impact **cross-module / contrat Rust interne**, limité à Hive. Aucune nouvelle route, DTO public, version d'événement, relation FGA, migration, dépendance, modification SDK/compose/monolithe. Telegraph, maintenance runtime, mTLS et nouvelle politique RBAC sont hors scope. Aucun commit/push/CI. L'exécution QA reste au lead selon le bail existant ; ce document n'autorise pas son auteur à lancer Cargo/runtime.

## 2. Annulation : organisation obligatoire jusqu'au service

- `application/src/command/invitation.rs` transmet **les deux UUID** à `InvitationUseCase::cancel_invitation`, puis `InvitationService::cancel_invitation` (`domain/src/service/invitation_service.rs`). Signature des deux ports : `cancel_invitation(&self, organization_id: Uuid, invitation_id: Uuid)` ; retours inchangés. Aucun repli public unscopé.
- Le repository `find_by_id` peut rester inchangé. Après lecture, le service vérifie `invitation.organization_id == organization_id` **avant** statut, `cancel()` et sauvegarde. Absence et mauvaise organisation produisent le même `EntityNotFound`, sans mutation ni divulgation d'organisation/email/token/statut étrangers. Acceptation par token inchangée.
- Le mapping actuel transformerait aussi ce refus en 500. Corriger **seulement l'annulation** : le command handler distingue `ApplicationError::Domain(EntityNotFound { .. })`, produit le code fermé `invitation_not_found` avec message générique ; `http/src/handlers/invitations.rs::cancel_invitation` mappe ce code vers `HttpError::NotFound` (404). Pas de matching de message, ni d'erreur DB convertie en 404. Conserver les autres mappings, notamment les 500 caractérisés par les tests d'état invalide. La garde HTTP `Write` sur `organization_id` demeure obligatoire.

## 3. Membres : total de collection, API de pages conservée

- Ajouter à `MemberService` : `count_members(&self, organization_id: Uuid) -> Result<i64, DomainError>`. Son unique implémentation délègue à **`OrganizationMemberReadRepository::count_by_organization` déjà présent**. Aucun nouveau port repository, pas de chargement de toute la collection pour compter.
- `application/src/usecase/member.rs::list_members` utilise ce total pour `total_items`, le plafond `total_pages`, `has_next` et les curseurs. Même population que la liste : **organisation exacte, tous les statuts et tous les issuers**. Le repository ne filtre pas `Active` : `count_active_by_organization` serait faux. Aucun nouveau filtre.
- Le DTO public est 1-based (`PaginationRequest::page`, défaut 1), le repository est 0-based (`fetch_page`). Convertir **une fois dans le use case à l'appel du service**, avec `page.saturating_sub(1)` ; service/repository restent 0-based, réponse/curseurs restent 1-based. Ajouter `Id` comme second tri après `CreatedAt DESC` dans `OrganizationMemberReadRepositoryImpl::find_by_organization` pour départager les timestamps égaux.
- Conserver les champs/conventions actuels : taille défaut 20/plafond 100, curseurs numériques de page, garde arithmétique de taille nulle, page hors plage vide mais total réel. Pas de nouvelle interprétation du champ entrant `cursor`, ni refonte des entrées invalides. Liste et count restent deux lectures usuelles : pas de promesse de snapshot sous mutations concurrentes ou de stabilité entre pages sous insertions/suppressions concurrentes.

## 4. Rôles : writer transactionnel unique

### Port et use case

Ajouter à `application/src/usecase/mod.rs::HiveOutboxUnitOfWork`, sur le modèle d'`add_member`/`remove_member` :

```rust
async fn replace_member_roles(
    &self,
    organization_id: Uuid,
    user_id: Uuid,
    issuer: String,
    roles: Vec<RolePermission>,
    event: Box<dyn DomainEvent + 'static>,
) -> Result<OrganizationMember, ApplicationError>;
```

Dans `MemberUseCaseImpl::update_member` :

1. Garder conversion complète des rôles, validation du principal cible et `require_can_assign_roles` **avant effet**. Exiger l'UoW ; absence = échec avant mutation/publication, sans fallback additif. `setup/src/app.rs:354–370` l'injecte déjà, sans nouveau wiring.
2. Construire `HiveDomainEvent::MemberRolesUpdated` avec l'ensemble canonique demandé et le principal **du membre cible persisté**, jamais l'issuer de l'administrateur. Une lecture du membre pour cette enveloppe est possible, mais le writer doit recharger/revalider la cible dans sa transaction. Le lookup issuer courant est exact, sans alias URL. L'événement est interne à l'application, pas une entrée HTTP. Le `Box<dyn DomainEvent>` suit le port existant et permet le rollback par `BadEvent` sans nouveau harness.
3. Appeler l'UoW une seule fois ; **ne plus appeler ensuite** `publish_member_roles_updated_event` / `record_event`. Un enregistrement dans une seconde transaction ou un envoi après commit ne remplit pas 0301. Construire l'événement en mémoire n'est pas le publier.

Retirer **`MemberService::update_member_roles` et son implémentation de trait** : son unique consommateur est ce use case, déplacé vers l'UoW. Le helper privé homonyme sert aussi à `add_member` : conserver ce chemin d'affectation initiale (nom/doc explicites si renommé). **Ne pas transformer `RoleService::add_roles` en remplacement**, ce qui changerait les parcours d'ajout/invitation. Aucun second chemin public de remplacement non transactionnel.

### Transaction dans `infra/src/transaction.rs`

1. `begin()` writer ; **toutes** les opérations suivantes utilisent `&txn`, jamais une connexion capturée par un repository hors transaction.
2. Vérifier l'organisation et trouver le membre par `(organization_id, issuer exact, user_id)`, sans fallback UUID-only ; refuser absent/ambigu, préserver `MEMBER_PRINCIPAL_AMBIGUOUS`. Verrouiller sa ligne avant toute suppression (`lock_exclusive`, idiome existant de `delete_organization`) et revalider son identité. Pas de lecture replica suivie d'un write aveugle.
3. Résoudre/valider **tout** l'ensemble des rôles avant suppression. Dédupliquer par `(organization_id, resource, permission)` ; chaque rôle doit exister dans l'organisation cible, sans rôle étranger/manquant/surplus silencieux. `find_by_organization_roles_with_connection` utilise deux `IN` indépendants : filtrer ses candidats par **couples exacts** et vérifier l'ensemble, ou employer un helper local exact ; ne pas persister le produit croisé, ni modifier implicitement ses autres callers.
4. Supprimer les liens du **member_id verrouillé seulement**, puis insérer une fois chaque grant résolu. Helper autorisé : `MemberRoleWriteRepositoryImpl::delete_by_organization_member_with_connection<C: ConnectionTrait>`. Ne pas supprimer les rôles de toute l'organisation.
5. Appliquer `OrganizationMember::update_roles` et sauvegarder le membre existant dans `&txn`. Retourner son nouvel ensemble : le mapper de sauvegarde remet actuellement `roles` à vide, donc réattacher les grants retournés. Préserver ID, organisation, principal, statut et dates d'origine ; ne jamais recréer une membership disparue.
6. `record(&txn, event.as_ref())`, puis `finish(txn, result)` : succès seulement après commit. Erreur de résolution/insertion/sauvegarde/sérialisation/outbox = rollback des liens, métadonnées **et** outbox. Aucune émission directe SQS, même avec `queue.enabled=false`.

`roles=[]` est l'ensemble vide ; aucun minimum/maintien du dernier owner n'est inventé ici. Demande dupliquée ou PATCH répété : aucun doublon DB. Les IDs des liens/timestamps et le nombre d'événements ne sont pas une clé d'idempotence HTTP ; chaque PATCH réussi peut enregistrer son événement.

### Concurrence et limites

Deux remplacements du même membre se sérialisent via la **ligne parent**, pas seulement les liens existants (l'ensemble peut être vide). Après deux succès, état exactement égal à un des ensembles complets, jamais une union/mélange ; chaque transaction a son propre événement cohérent. Une suppression concurrente attend ou fait échouer/annuler le remplacement, jamais ne permet sa recréation. Erreur/deadlock DB = erreur avec rollback, pas succès partiel ou retry maison.

Aucune nouvelle lease, mutex process-local, isolation globale ou migration d'unicité. Pas de promesse d'ordre de livraison, d'exactly-once ou de convergence FGA sous livraison réordonnée. Le TOCTOU des contrôles d'autorisation d'un autre acteur n'est pas rendu atomique par ce verrou cible ; conserver les contrôles existants, sans annoncer sa résolution. Une extension de ces garanties exige une mission distincte.

## 5. Consommateurs vérifiés / ownership

Serena sur racine canonique, HEAD/diff source Hive vérifiés ; recoupement exact sources/tests :

- **InvitationService : 3 impls**, 1 prod (`domain/src/service/invitation_service.rs`), 2 probes (`domain/src/service/sync_service_principal_tests.rs:384`, `application/src/usecase/usecase_sync_job_tests.rs:414`) dont adapter les signatures. 1 appel métier d'annulation dans `application/src/usecase/invitation.rs`.
- **InvitationUseCase : 1 impl**, 1 référence d'annulation dans `application/src/command/invitation.rs`. `CancelInvitationCommand` porte déjà l'organisation ; aucun nouveau constructeur HTTP.
- **MemberService : 1 impl**, `domain/src/service/member_service.rs`, à compléter pour count. **1 appel métier** au mutateur retiré dans `application/src/usecase/member.rs`. Le helper privé a aussi l'appel d'ajout initial à préserver. Les MemberService Manifesto sont des homonymes, pas des impls Hive.
- **HiveOutboxUnitOfWork : 2 impls**, prod `infra/src/transaction.rs` et probe `application/src/usecase/usecase_sync_job_tests.rs:534` à compléter mécaniquement. HIR `who_uses` : **11/11 références non-import**, 6 modules (`external_link`, `invitation`, `sync_job`, `member`, `organization`, `infra::transaction`). HIR omet probes et `tests/outbox_tests.rs`, retrouvés par Serena/sources ; il ne remplace pas la compilation.
- Composition `setup/src/app.rs` et autres use cases consommateurs inchangés, sauf adaptation de compilation réellement nécessaire. Aucun consommateur externe supplémentaire détecté par refs/recoupement ciblé `services/`, `crates/`, `workers/` ; pas une preuve pour arbre externe, cfg inactif, macro ou worktree non mergé.

**Un seul auteur production Hive** possède ensemble domaine/application/infra, mapping HTTP d'annulation et adaptations mécaniques des deux probes. Helpers nouveaux infra seulement, `ConnectionTrait` existant, aucun type SeaORM dans les ports. Un auteur QA peut posséder les deux fichiers `coverage90_*` canoniques après gel des signatures ; le lead intègre et détient le slot Cargo. Ne pas séparer writer et use case entre auteurs concurrents. Auteur Telegraph sans recouvrement.

## 6. Validation requise au lead / auteur QA

Garder noms et oracles des binaries **`coverage90_invitation_transitions`** et **`coverage90_member_roles`** ; ni ignore ni affaiblissement de leur ROUGE exécuté. Extensions dans ces mêmes binaries, fixtures réelles existantes, `#[serial]` pour les singletons :

- Annulation légitime ; mauvaise org autorisée en Write → 404 + ligne étrangère strictement identique ; ID absent → même 404 générique ; garde Write et transitions conservées.
- 5 memberships / taille 2 → 2,2,1 + union exacte, total 5 / 3 pages, curseurs précédent/suivant ; autre org exclue ; Pending/Suspended inclus autant que dans le count ; page hors plage et timestamps égaux. Zéro vérifiable au repository avec vraie DB si l'HTTP exige lui-même une membership du caller.
- Read→Write puis Write→Read sans résidu ; même PATCH répété et demande dupliquée sans doublons ; ensemble vide ; rôle invalide après un valide sans effet ; org/issuer cible et ID membership conservés. Dédupliquer les tests déjà présents (répétition PATCH déjà dans `coverage90_member_roles.rs`).
- **Rollback après mutation** : vraie nouvelle UoW + pattern `BadEvent` de `tests/outbox_tests.rs` (échec `to_json` au record, après remplacement). Vérifier liens et métadonnées identiques à l'avant, zéro nouvel outbox. Témoin positif : ligne `member_roles_updated` pending avec ensemble exact et issuer cible. L'invalidité upfront seule ne prouve pas ce rollback.
- **Concurrence DB réelle** : deux remplacements, connexions distinctes, chevauchement provoqué/observé par verrou DB et synchronisation bornée, pas `sleep` ou appels possiblement séquentiels ; état final complet unique sans union/doublons. Le retrait concurrent ne doit pas ressusciter le membre ; ne pas prétendre ce scénario couvert par le seul test à deux PATCH.

`BadEvent` ne simule ni serveur, use case ni PG : seule sa sérialisation échoue dans la vraie transaction. Pas de harness/failpoint production ou mocks PG/OpenFGA nouveaux.

Après intégration : fmt/Clippy ciblés, compilation cibles Hive concernées (dont les probes via tests domaine/application), puis les deux binaries via `ops/tests/it-native.ps1 -Suite hive -TestBinary <nom-exact>`. Régressions existantes utiles : `invitation_api_tests`, `member_update_api_tests`, `members_api_tests`, `outbox_tests` ; **ne pas** passer ces noms à `-TestBinary`, sa liste actuelle ne les autorise pas. Le lead arbitre le passage suite Hive sans filtre s'il les exécute, sans bypass ni élargissement implicite de liste. Windows natif, jobs 24 du dépôt, target E:, un Cargo, runtime/nettoyage tracé selon ses autorisations. Revues correctness/sécurité ciblées prévues par QA. Aucun test/runtime exécuté par l'architecte ; pas de preuve FGA/queue E2E déduite.

## 7. Sources / contradictions / limites

- Annulation : `application/src/command/invitation.rs:199–204`, `application/src/usecase/invitation.rs:247–254`, `domain/src/service/invitation_service.rs:331–363`, `http/src/handlers/invitations.rs:52–80` ; 0402 exige déjà le scope org, non garanti par la garde seule sur un objet chargé globalement.
- Pages : `application/src/usecase/member.rs:412–464`, `application/src/dto/common.rs:93–114`, `infra/src/repository/organization_member_repository.rs:218–242,292–302`. Analogue, pas copie du DTO : `services/Manifesto/application/src/usecase/member.rs:497–533` compte sa population filtrée via son service.
- Rôles : `domain/src/entity/organization_member.rs:136–140`, `domain/src/service/member_service.rs:155–170,273–287`, `domain/src/service/role_service.rs:231–258`, `application/src/usecase/member.rs:486–529` : mémoire/event remplaçants mais DB additive et outbox après writes séparés. Écart à 0301 pour ce parcours, pas nouvelle cible.
- Downstream inchangé : `crates/hive-events/src/member.rs:53–65` ; `workers/sentinel-sync/src/translator/hive.rs:133–143` supprime les relations anciennes puis écrit le nouvel ensemble. Lecture statique, pas consommation FGA exécutée dans cette campagne.
- UoW : `infra/src/transaction.rs::begin/finish/add_member/remove_member/delete_organization`, helpers `save_with_connection` / `delete_by_organization_with_connection`. Verrou parent déjà présent dans la suppression d'organisation.
- Wiki QMD `obsidian/AI FOR ALL/projects/hive/concepts/invitation-driven-membership.md` (`#45ef6f:22–36`) annonce encore POST seul/réponse avec token : conception périmée face à 0402/handbook/code, pas contrat à restaurer. 0314 reste canoniquement `Accepted / Unimplemented` malgré des mécanismes exact-issuer présents : aucune promotion globale de sa Réalité ici.
- Aucun Cargo/test/runtime/CI/scan exécuté par l'auteur ; rouges repris des preuves du lead. LSP/HIR sur source canonique, pas certification des worktrees externes. Correctif à implémenter/qualifier ; aucun PASS global, Accept, commit ou publication implicite. Pas d'ADR écrite, donc aucun digest Serena à synchroniser.
