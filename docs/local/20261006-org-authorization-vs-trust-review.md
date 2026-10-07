# Organisation métier, identité et autorité de signature — revue bornée

Date : 2026-10-06. **Analyse / proposition non contraignante — aucune décision ratifiée par cette note.**
Périmètre : critique de la proposition ChatGPT, contrats IAM/Hive et consommateurs proches Apparatus/Lazaret. Pas une lecture exhaustive de tous les ADR du dépôt.
Repères fournis par le parent : root f5c0ba7 / SDK 4cdf202 ; pas de revalidation Git. Relecture statique des sources seulement ; aucun test, Cargo, runtime, installation, commit ou push.

## 1. Besoin produit qui reste applicable

Particuliers et entreprises : autorité plateforme commune par défaut ; clé dédiée uniquement sur demande d'une entreprise, éventuellement dans son propre KMS. Activation rare. Aucun report/retrait de cette fonctionnalité n'a été ratifié.
La question utilisateur demande un avis : elle n'autorise pas à supprimer ce besoin. Une ADR Accepted n'est pas à elle seule une preuve commerciale ; le besoin explicite utilisateur est ici une preuve distincte. Un consommateur absent aujourd'hui ne rend pas une exigence prochaine inexistante.

## 2. Verdict indépendant

**Accord** : Organization ≠ Identity ≠ SigningAuthority ; le multi-org ordinaire, des claims contextuels et la fédération SSO n'exigent pas chacun une clé/identité/issuer distinct. Le parcours utilisateur plateforme sans réémission par org est déjà le contrat du dépôt.
**À réexaminer** : le couplage garde du KMS entreprise → autorité autonome → issuer/OrganizationManagedIdentity distincts. Ce n'est pas une obligation JWT ; une révision pourrait satisfaire le besoin avec moins de surfaces, mais doit prouver l'isolation et changer explicitement les contrats concernés.
**Désaccord** : retirer en bloc org signing/BYOKMS, lifecycle et admission comme si le besoin entreprise avait disparu. Ce serait un retrait produit, pas une simple optimisation.
**Option non démontrée nécessaire** : token exchange/context token et cache dédié. Ne pas construire un nouveau STS pour remplacer un problème déjà résolu par l'AuthZ de ressource.

## 3. Canon : Statut et Réalité, tels que lus

- 0304 `Accepted / Partial` : séparation issuer, owner, opération crypto, publication ; clé commune/dédiée/BYOKMS et issuer par trust domain. `docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md:21–34`.
- 0305 `Accepted / Implemented` : une PlatformIdentity multi-org sans réémission ; OrganizationManagedIdentity explicite lorsque le trust domain change. La photographie Implemented documente elle-même les gaps linking/switch/mint JWT org. `docs/adr/0305-account-identity-trust-domain.md:12,20–38,75`.
- 0306 `Accepted / Partial` : configuration admin Hive → IAM ; HTTP et métadonnées/rotation présents, BYOKMS cloud et events encore gaps. `docs/adr/0306-hive-iam-configuration-signature.md:12,22–36`.
- 0307 `Accepted / Implemented` : port WorkloadIdentity pour credentials WIF/s2s ; pas une identité utilisateur ni un choix imposé SPIFFE. `docs/adr/0307-workload-identity-port.md:12–29`.
- 0308 `Accepted / Partial` : mesh AuthN/principal et révocation/fraîcheur bornée ; ne pas confondre avec l'AuthZ métier. `docs/adr/0308-mesh-authn-jwt.md` (Décision, notamment point 6).
- 0309 `Accepted / Partial` : port minimal Sign/GetPublicKey ; adapter HTTP digest-only et tests wiremock, vendor HSM absent. `docs/adr/0309-remote-signer.md:12–27`.
- 0310 `Accepted / Partial` : admission writer atomique et nombres ratifiés, pas une autorisation de relâcher une protection. `docs/adr/0310-admission-epochs-budget-jwks.md:21–27,45`.
- **Aucun fichier 0311** trouvé dans `docs/adr/` lors de cette lecture ; index indique 0311+ comme prochain libre. Ne pas citer une ADR0311 inexistante comme contrat.
- 0104 `Proposed / Partial` : overrides de ports sortants au composition root ; le remote signer est explicitement hors décision. `docs/adr/0104-outbound-overrides-composition-root.md:3–4,12,32`.
- 0200 `Accepted / Implemented` : IT avec l'infrastructure réelle du harness ; aucune permission de remplacer ces gates par des mocks métier ou de supprimer des assertions. `docs/adr/0200-it-infra-reelle-rustycog-testing.md`.

## 4. Consommateurs : présent, prévu, non prouvé

- **Humain plateforme, présent en source** : émission `org=None`, issuer/audience plateforme ; require_active_signing_epoch refuse scope non-Platform, organisation de clé et organisation de claims. `services/IAMRusty/infra/src/token/jwt_encoder.rs:196–239,603–631`. La configuration d'une clé org ne sélectionne pas déjà son signer au login/refresh.
- **Hive, présent en source** : membership issuers et principal `(iss,sub)` ; organisation de ressource/path vérifiée par OpenFGA. `docs/adr/0305-account-identity-trust-domain.md:27,34`; `services/Hive/http/src/lib.rs:21–24,62–81`. UI active-org n'est pas une autorité.
- **Confiance org, administration/persistance présentes, émission humaine incomplète** : signer configure/test/rotate/disable et Identity RPC existent ; pas d'UX switch ni mint org livré démontré. `docs/adr/0306-hive-iam-configuration-signature.md:36`; `docs/adr/0305-account-identity-trust-domain.md:34–38`.
- **Barrière compte plateforme, test présent non réexécuté ici** : token org légitime avec sub victime doit rester403 sur me/link/relink et sans effet OAuth. `services/IAMRusty/tests/organization_account_guard.rs:93–133`. Supprimer cette frontière n'est pas retirer un test arbitrairement coûteux.
- **Apparatus/Lazaret P3, Accepted / Implemented au canon, source présente** : mTLS puis session courte avec clé dédiée, pas IAM ; identité instance/binding/release/generation/grant_revision, jamais utilisateur. `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md:29,53–60`; `services/Lazaret/domain/src/identity.rs:30–43,72–93`. Port s2s0307 et identité runtime Lazaret sont deux préoccupations distinctes.
- **AuthZ Apparatus, pas OpenFGA seul** : contrôles live Manifesto de binding, consentement et grants ; fermeture au commit DB, projection FGA insuffisante. Donc « OpenFGA est seul responsable de toute AuthZ de la plateforme » est excessif. `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md:29`.
- **P4 proche, mécanisme encore Proposed / Unimplemented** : Pod projet×binding et I/O Lazaret ; partagé/organisation hors V1. `docs/adr/0011-apparatus-p4-pod-par-binding.md:3–4,20–23`.
- **P4–P6 ouverts** : Factory, host UI, iframe, MessageChannel et pipeline OCI ne sont pas des consommateurs livrés à inventer. `docs/adr/0007-apparatus-p3-capability-boundary-after-accept.md:292`.
- **Conclusion de portée** : ces plans prouvent le besoin workload/capabilities, pas le besoin de clés BYOKMS organisationnelles pour ces workloads. Aucun consommateur précis BYOK dans ces plans ciblés n'a été démontré ; le besoin entreprise explicite reste néanmoins applicable. Pas d'affirmation que tous les plans possibles ont été exclus.

## 5. Invariants et contradictions à ne pas lisser

Garde/custodie du KMS, issuer, namespace de principal et autorité de confiance sont des axes distincts. Clé dédiée ou nouveau kid seul n'isole rien : il faut contrôler key-owner/issuer/scope et le rattachement à la ressource autorisée. Garder un issuer commun nécessiterait un contrat explicite protégeant notamment les comptes plateforme ; pas simplement supprimer `(iss,sub)`/OrganizationManagedIdentity.
SSO d'entreprise = source d'authentification ; ce n'est pas nécessairement une autorité de signature de JWT AIForAll. Un KMS autorisé à Sign ne décide pas des claims ou permissions et n'empêche pas la plateforme d'obtenir les signatures permises par ses credentials.
Le canon0304 choisit effectivement une autorité org capable de créer des principals de son domaine (`:26`), un issuer par trust (`:29`) et `org` comme contexte de trust, pas permission (`:31`). La proposition de retirer ces éléments le contredit ; une critique de leur nécessité reste légitime, mais pas une réécriture implicite.
**Ruling strict plateforme/org** : `.cursor/review-briefings/20261005-session-standards.md:26–28` demande403 sans headers principal si token plateforme porte `org`, et documente les alignements restants.
**Écart source constaté** : le SDK consommé `verify_rs256` contrôle org-owner seulement lorsque la clé a un owner org, puis `claims_to_principal` recopie `org` sans distinction. `rustycog/rustycog-http/src/jwt_handler.rs:428–440,534–582`. `workers/ext-authz/src/check.rs:114–126` délègue à cet extractor. Ne pas annoncer strict-reject uniformément livré ; aucune correction faite ici.
`org_id` et `org` ne sont pas le même champ ; une extension non réservée n'est pas une autorité. Renommer un claim ne remplace pas un contrat de jeton/context/purpose/audience et ses règles de consommation. Audiences dédiées sont encore Non décidé0304 (`:27`), pas déjà adoptées.
Révocation membership, session et clé sont distinctes (`0305:29`). TTL court, offline et cache de claims ne garantissent ni membership live, ni propagation automatique de révocation.

## 6. Simplifications autorisables après contrat, pas suppressions validées

Priorité proposée : crypto privée déléguée derrière SigningProvider ; éviter conversions/publications/accounting répétés ; préparer des snapshots réutilisables par révision/transition/expiration ; limiter la matrice de providers effectivement livrés. Le port minimal et les coutures existent déjà (0304/0309) : pas besoin d'un framework universel supplémentaire.
Supprimable sans retrait fonctionnel seulement après preuve d'équivalence : doublons et travail redondant, pas les barrières de trust. Toute optimisation conserve writer atomicité, concurrence inter-org, epoch fence, preuve public/privé, N/N+1, Pending/Active/Retiring/Revoked, publisher canonique et fraîcheur maximale60s.
Les quotas sont des protections d'un JWKS global :786432 octets totaux,65536 réservés plateforme,720896 global org,8 epochs/org et4 nouveaux kids/3600s. `0310:24–25`. Le720 de la boucle de test n'est ni720 entreprises ni un quota produit. Rare activation n'écarte pas DoS/abus ou erreur d'administration.
Changer la borne/son algorithme requiert une preuve de même protection et, si la cible ratifiée change, un accord humain. Retirer tout sign-org peut retirer ce risque spécifique, mais uniquement après décision explicite de retrait produit/API, migrations et nouvelle matrice de tests.
Aucun modèle/routes/configure-test-rotate-disable/admission ni scénario existant n'est autorisé à disparaître par cette note. Les coûts CI ne démontrent pas l'inutilité du produit ; pas de timings par phase ni gain promis ici.
Retirer org signing ne supprime pas le chemin privé PEM plateforme ni ne remédie par lui-même à l'advisory RSA ; verdict sécurité déjà porté par `.tmp/20261006-rustsec-rsa-assessment.md`, pas réinvestigué ici. Les401 JWKS froid du handoff sont également un sujet indépendant ; aucune nouvelle exécution n'a validé leur résolution.

## 7. Standards : limites déjà vérifiées ailleurs, pas de nouvelle recherche

Auth0 Organizations montre issuer tenant + contexte org_id et validation/partition attendue ; pas preuve d'absence générale de BYOK ou de membership live. RFC9068 rend client_id/iat/jti/typ at+jwt obligatoires SI ce profil est adopté ; adoption repo non démontrée. RFC8693 ne garantit pas même issuer/clé, TTL court, narrowing ou révocation propagée. Ne pas importer ces garanties dans une ADR par citation seule.

## 8. Blast radius, décisions humaines et contrat de suite

Impact d'un retrait : **architectural / API-contrat / cross-module**. IAM domaine/identities/signing registry/issuer, application/mint/refresh/admin, migrations et credentials ; Hive metadata/clients/routes/events ; SDK JWT/trust, ext-authz/JWKS ; standalone et pont `runtime/monolith/src/in_process_iam_signer.rs` ; tests de garde compte, publisher, rotation, admission et transport. Pas une optimisation locale ni une raison de toucher au protocole workload Lazaret.
**Décision1** : maintenir clé entreprise/BYOKMS dans le prochain périmètre livré, ou reporter explicitement avec condition/date de réintroduction. Aucune décision de report présumée.
**Décision2** : quels jetons portent la clé entreprise (humains/intégrations/workloads), et s'agit-il seulement de custodie/signature dédiée ou d'une autorité org créant ses propres identités ? C'est ce qui détermine nécessité d'issuer/Identity distincts.
**ADR** : aucune créée/modifiée ici. Optimisation bornée par Accepted → pas d'ADR nouvelle ; changement de trust/public API/retrait → draft forward Proposed puis ratification humaine, amendements/supersession ciblés0304/0305/0306/0310 et audit0308/0309.0311 est un candidat libre constaté, à revérifier au moment du draft. Aucun statut « Future » inventé et aucune Accept autonome.
**Contrat implementer futur** : ne commencer qu'après scope choisi ; ne pas préempter ces décisions pendant une correction perf/JWKS. Préserver les scénarios retained et assertions ; un retrait produit ratifié exige sa propre migration et ses propres tests. Validation ciblée via suites du dépôt (IAM organization_account_guard/jwks/signing_admission/signing providers, SDK jwt/trust et ext-authz, Hive signer/membership/bridge selon impact), pas « tout cargo » ; aucune validation lancée maintenant.
Sources complémentaires consultées : INDEX/briefs ciblés, handbook `docs/platform/authn-jwt.md`, QMD wiki Hive/Lazaret (conception, pas canon), analyses locales précédentes. Pas de relecture des logs froids ni de recherche Marvin/standards dupliquée.