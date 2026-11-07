# OAuthConnect : OIDC et service opérationnel

## Portée approuvée

GO du 2026-11-07 : « go oidc et service op ». Appliquer ADR0413 **Accepted/Partial**, contrat [multi-instances](../contracts/oauthconnect-multi-instances.md) §§2.4,3,4,6–7, et [frontière HTTP/secrets](../contracts/oauthconnect-http-secrets-boundary.md).

Base publiée : `eabc6459bc5f3f5c564d8789de68594226cb7aed`. La tranche précédente livre OAuth configurable, transports/secrets et composition OAuth-only, 189 tests et qualité ciblée vérifiés. Les preuves antérieures demeurent valides tant que leurs sources ne changent pas.

Cette tranche vise OIDC standard (RS256, issuer/claims/nonce/at_hash vérifiés, UserInfo borné), JWKS technique par instance, registry mixte complète immutable, récepteur v2 HMAC et service RustyCog démarrable. Réutiliser configuration/secrets/erreurs/framework existants ; pas de scripts, claims riches ou actions déléguées.

**Exclus** : IAM et migration transactionnelle, E2E GitHub/GitLab assignés à un autre intervenant, activation déploiement/cutover, suppression legacy, changements SDK/Compose/Kind. L'équivalence sub GitLab ancien ID et les E2E restent bloquants de bascule.

## Méthode et preuves attendues

- Direct avec sous-agents bornés, sans mission Ensemble/PM. TDD RED comportemental→GREEN par tranche, préflights distincts.
- Clarification architecture et QA en parallèle ; socle compilable refusant intégré avant consommateurs isolés. Puis tests OIDC et routing indépendants ; composition/service réel après intégration de leurs entrées.
- QA proportionnée au risque OIDC/HMAC : tokens réellement signés, WireMock sortant et service/listener réel, cas négatifs et isolation inter-instance, cache TTL/rotation/single-flight ; revues indépendantes bornées.
- Compilation/tests natifs Windows, slot Cargo unique et jobs24 du dépôt. Pas de Docker/Kind attendu ; tout besoin nouveau soumis aux garde-fous runtime.
- Parent intègre, vérifie le contenu retenu (fmt/check/Clippy/tests ciblés), actualise documentation, commit/push master et contrôle référence distante/nettoyage. Un gate absent demeure explicite, pas PASS global.

## État

Préparation en cours, aucune preuve nouvelle d'OIDC/service livré. Ledger détaillé local : `.cursor/handoffs/20261107-oauthconnect-oidc-service.md`. QA existante : `.cursor/qa-plans/20261106-oauthconnect-tdd.md`, T4/G-4 et T6/G-6 ; plan ciblé de cette tranche à réconcilier avant tests.
