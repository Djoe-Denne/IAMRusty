# ADR-0311 : En production IAM délègue les clés privées et signatures JWT aux providers

- Statut : Accepted
- Réalité : Implemented
- Date : 2026-10-08
- Décideurs : Djoé Denne ; périmètre approuvé le 2026-10-06, rédaction Proposed le 2026-10-07 ; Accept explicite le 2026-10-08, pas Accept autonome
- Jalon concerné : architecture actuelle / IAM signature ; hors P-Apparatus
- SuperSède : amendement ciblé de 0304 §4 sur la crypto privée production et le PEM dev/test, effectif après Accept explicite du 2026-10-08 ; pas de supersession de 0304 entière
- SuperSédée par : —
- Related :0304,0305,0306,0307,0308,0309,0312

## Contexte

**État historique à la rédaction du 2026-10-07.** SigningProvider, Transit et Remote existent. Le chemin PEM génère/charge encore du RSA privé ; Transit ne transmet pas explicitement key_version. Le produit conserve plateforme commune par défaut et autorité entreprise indépendante derrière KMS/HSM. L'accord utilisateur ne signifie ni livraison ni Accept formelle.

## Décision

- Génération, stockage/import privé éventuel et opérations privées de signature JWT sont délégués en production à OpenBao/KMS/HSM sans export privé vers IAM, y compris pour la plateforme. Réutiliser Sign/GetPublicKey et WorkloadIdentity ; aucun nouveau broker/framework.
- PEM privé reste uniquement dev/test explicite. Aucun fallback PEM ou plateforme après panne du provider entreprise. Retirer keygen/fichiers/crypto privée production seulement après validation provider plateforme ET org et migration maîtrisée.
- IAM conserve claims/header/assemblage, registry publique, credentials opaques, audit métier et binding owner/issuer/kid/alg/keyref/version. Une organisation/HSM enrôlée peut signer ses tokens uniquement dans son domaine ; jamais auto-trust d'un token étranger ni droits plateforme implicites.
- Public, probe et signature visent une version immuable épinglée ; Transit utilise key_version. Create/Rotate sont capacités facultatives : un HSM client peut fournir seulement Sign/public et proposer son nouveau public.
- Rotation : nouveau matériau/public validé et preuve de possession, Pending/prépublication, activation atomique, signature versionnée/fence, ancien public Retiring jusqu'au TTL+skew puis retrait/révocation. Rotate du provider seul n'active ni ne révoque les JWT hors ligne.
- Conserver PlatformIdentity multi-org sans JWT par changement UI, identité/trust org distincts, guards compte, authorisation admin, fraîcheur60s, TLS/egress/SSRF, moindre privilège, erreurs redacted et indépendance des workloads Lazaret. Pas de délégation totale OAuth/OIDC à Transit.

## Conséquences

Moins de garde/crypto privée maison, mais dépendance réseau/disponibilité/coût Sign. Pas de gain chiffré ni de PASS sécurité implicite. Préflight des profils/bindings PEM, prépublication du remplaçant, activation vérifiée avant retrait privé ; préserver les anciens publics. Rollback uniquement vers provider/version encore autorisé, jamais clé révoquée ou downgrade PEM silencieux. Les tests provider réels, mismatch/version, N/N+1, revoke tardif et guards restent ; stubs seuls insuffisants. Vendor HSM et nouvelle UX/mint org restent hors décision ; aucune matrice cloud complète imposée.

## Références

- `services/IAMRusty/domain/src/port/signing.rs:18–40` ; `services/IAMRusty/infra/src/signing/rotate.rs:153` ; `services/IAMRusty/setup/src/app.rs:1017,1036,1196–1208` ; adapters PEM/Transit/Remote.
- Canon0304–0309 ; `docs/local/20261006-signing-simplification-inventory.md` et `20261006-org-authorization-vs-trust-review.md`.
- Réalité Implemented : rotation déléguée via `rotate_provider_material()` (`services/IAMRusty/infra/src/signing/rotate.rs:153`), version Transit épinglée (`require_transit_key_version` dans transit/probe/scoped et signing_key_registry), PEM limité au dev/test explicite et binding plateforme Transit sans fallback (`services/IAMRusty/setup/src/app.rs:1017,1036,1196–1208`).

## Réalité courante — 2026-10-08

- Ratification explicite de Djoé Denne : **Accepted / Implemented**. L'implémentation du paquet signing a été livrée après la rédaction Proposed (commits `0da64c2`, `8c582d4`, `1d66967`, `f33cf2a`, `6296d0f`, `2dc4126` ; preuves source vérifiées par le contrôleur au HEAD `e8fcaea`).
- Rotation : `services/IAMRusty/infra/src/signing/rotate.rs:153` appelle `rotate_provider_material()` ; plus de keygen RSA local en production. Public/probe/signature exigent `require_transit_key_version` dans `infra/src/signing/{transit,probe,scoped}.rs` et `infra/src/repository/signing_key_registry.rs` (préfixe `services/IAMRusty/`).
- PEM : `services/IAMRusty/setup/src/app.rs:1017` utilise `config.security.mode.allows_local_pem()` ; les fixtures exigent `IsolatedTest` (`app.rs:340–350`). La plateforme impose Transit sans fallback (`app.rs:1036`) et `provider_type == OpenBaoTransit` (`app.rs:1196–1208`).
- Suites IAM **unitaires + IT vertes sur master**, exécutées et confirmées par Djoé Denne le 2026-10-08 ; notamment `signing_epoch_writer`, `transit_protocol`, `signing_admission`. Il s'agit d'une **attestation utilisateur**, pas d'un artefact CI archivé ni d'une exécution par cet agent. Elle ne couvre pas l'E2E exact-route/Envoy de 0412 et ne vaut pas PASS sécurité global implicite.
