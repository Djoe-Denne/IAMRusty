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

Schéma pédagogique conservé : [HTML autonome](../diagrams/oauthconnect-architecture.html) · [notice et sources](../diagrams/oauthconnect-architecture.md). Photographie cible du 7 novembre, indépendante du serveur canvas ; ouverture locale et impression PDF possibles, pas une preuve de livraison.

Complément demandé sur les clés : [schéma OIDC/JWKS autonome](../diagrams/oidc-keys-responsibilities.html) · [stockage, rotation, responsabilités et constats de déploiement](../diagrams/oidc-keys-responsibilities.md). Il sépare les clés privées du fournisseur, le cache public RAM Connect, les secrets client/HMAC et les clés JWT IAM ; emplacement privé GitLab et runtime actif restent à confirmer côté Ops, pas déduits des manifests.

Travail en cours, **OIDC et binaire opérationnel non encore qualifiés**. Le socle refusant a été intégré en `72459ea` avec `openidconnect4.1.0` verrouillé et ses API recoupées ; compilation/fmt/Clippy strict ciblés et42régressionsOAuth passent.

Le router registry immuable est intégré en `867a47f` : **18 tests passent**, après14RED comportementaux qualifiés et4caractérisations. Ils emploient de vrais clientsOAuth, le récepteurv2 existant, les clés par instance et le PATHOriginalUri complet ; routes non canoniques/aliases, extraction, erreurs, readiness et listenerSDK possédé port0/arrêtjoint sont couverts. ClippyHTTP strict passe. Cela ne qualifie ni le boot mixte, ni OIDC, ni le binaire ou un fournisseur live.

OIDC est désormais implémenté et vérifié localement : **40testsOIDC+20régressionsbudget=60PASS**,0ignoré/filtré, après37RED/3PASS au socle. Le premier candidat passait58/60 : les2positifsUserInfoJWT étaient servis en text/plain par les fixturesWireMock ; la réparation de leur MIME et un préflightHTTP renforcé ont permis de passer les mêmes oracles **sans assouplir la production**. Clippy strict4paquets passe. Revue tests indépendante favorable ; revue sécuritéA+B sansCRITICAL/HIGH/MEDIUM, unLOW (queryauthorize réservée) retenu pour correctionTDDavantclôture.

Le boot mixte, Application/listener et véritable binaire restent la prochaine tranche C ; **serviceopérationnel, IAM, fournisseur live et déploiement non qualifiés**. Ledger détaillé local : `.cursor/handoffs/20261107-oauthconnect-oidc-service.md`. Plan ciblé réconcilié : `.cursor/qa-plans/20261107-oauthconnect-oidc-service.md`, en continuité de QA20261106 T4/G-4 etT6/G-6. Les preuves/gatesrestantes sont bornées au lot, sans PASS global anticipé.
