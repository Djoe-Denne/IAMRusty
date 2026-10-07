# ADR-0311 : En production IAM délègue les clés privées et signatures JWT aux providers

- Statut : Proposed
- Réalité : Partial
- Date : 2026-10-07
- Décideurs : Djoé Denne ; périmètre approuvé le2026-10-06, statut formel Proposed demandé
- Jalon concerné : architecture actuelle / IAM signature ; hors P-Apparatus
- SuperSède : aucune effective ; amendement ciblé de0304 §4 après Accept explicite
- SuperSédée par : —
- Related :0304,0305,0306,0307,0308,0309,0312

## Contexte

SigningProvider, Transit et Remote existent. Le chemin PEM génère/charge encore du RSA privé ; Transit ne transmet pas explicitement key_version. Le produit conserve plateforme commune par défaut et autorité entreprise indépendante derrière KMS/HSM. L'accord utilisateur ne signifie ni livraison ni Accept formelle.

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

- `services/IAMRusty/domain/src/port/signing.rs:18–40` ; `infra/src/signing/rotate.rs:197–260` ; `infra/src/signing/transit.rs:187–198` ; adapters PEM/Transit/Remote.
- Canon0304–0309 ; `docs/local/20261006-signing-simplification-inventory.md` et `20261006-org-authorization-vs-trust-review.md`.
- Réalité Partial : ports/adapters source présents ; retrait PEM production et orchestration versionnée complète non livrés/non testés ici.