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

Le binaire est maintenant **fonctionnel et qualifié localement sur fixtures**, pas déployé. Le socle refusant a été intégré en `72459ea` avec `openidconnect4.1.0` verrouillé et ses API recoupées ; compilation/fmt/Clippy strict ciblés et42régressionsOAuth passent.

Le router registry immuable est intégré en `867a47f` : **18 tests passent**, après14RED comportementaux qualifiés et4caractérisations. Ils emploient de vrais clientsOAuth, le récepteurv2 existant, les clés par instance et le PATHOriginalUri complet ; routes non canoniques/aliases, extraction, erreurs, readiness et listenerSDK possédé port0/arrêtjoint sont couverts. ClippyHTTP strict passe. Cela ne qualifie ni le boot mixte, ni OIDC, ni le binaire ou un fournisseur live.

OIDC est désormais implémenté et vérifié localement : **41testsOIDC+20régressionsbudget=61PASS**,0ignoré/filtré, après37RED/3PASS au socle et unRED additionnel pour la queryauthorize. Le premier candidat passait58/60 : les2positifsUserInfoJWT étaient servis en text/plain par les fixturesWireMock ; la réparation de leur MIME et un préflightHTTP renforcé ont permis de passer les mêmes oracles **sans assouplir la production**. Le LOW sécurité sur les paramètres réservés Discovery est corrigé en `4c9a4b2`, revue indépendante et test exactGREEN acquis ; politique OAuth partagée,38régressionsconfiguration et18routing passent. Clippy strict5paquets passe. Aucun CRITICAL/HIGH/MEDIUM identifié dans la revue A+B.

La tranche C est implémentée en `462ccc5` : **16tests passent** (1privé,11composition,4vrai binaire), après10RED/5PASS au socle, sansignorés/filtrés dans les suites complètes. Elle couvre catalogue OAuth/OIDC atomique, secrets directs/coffre SDK acquis une fois, HTTPcomposé gzip/CA/SAN, échecs tardifs sans écoute, port0/adresseeffective, HMACparinstance et profilnormalisé, rebuildsanscorrélation, arrêt/join propriétaire SDK. Le binaire natif Windows est réellement lancé avec configuration/CWD isolés ; son nettoyage utilise le fallback kill/reap du seul enfant, **pas une preuve de signal Windows gracieux**. La revue sécurité finale ne relève aucun finding. Les preuves refusantes §10 sont exécutées : aucun principal/UUID pour tokens vides, malformés, HS256 ou RS256 correctement signés de fixture ; un Bearer ne remplace pas le HMAC et ne déclenche pas de requête fournisseur.

### Campagne finale

Commande réelle : `cargo test --locked -p oauth-connect-configuration -p oauth-connect-domain -p oauth-connect-infra -p oauth-connect-http -p oauth-connect-setup -p oauth-connect-service -p idp-connect-contract --no-fail-fast` : **295PASS,0échec,0ignoré,0filtré**. `cargo fmt` ciblé sur les mêmes paquets (`-- --check`) et `cargo clippy` (`--all-targets -- -D warnings`) passent ; seul le warningSDKpréexistant `async_fn_in_trait` demeure dans la dépendance, SDK/pin inchangés. Les anciens tests OAuth-only sont compris dans cette campagne.

Une première compilation finale avait échoué sur des artefacts rlib/E0463 avant tout test. Le préflight verbose puis la campagne complète ont réussi sans nettoyage global, modification du SDK, de la toolchain ou des24jobs ; aucune causalité définitive de cet incident n'est revendiquée. Les logs originaux et validés sont conservés séparément. Publication et nettoyage de mission doivent être vérifiés distinctement avant annonce de clôture administrative.

La fixture TLS reprend les DN distincts CA/feuille déjà employés dans IAM et `c_tls`, sans désactiver CA/SAN. La correction `Send` de Discovery est distincte du réseau : adapter typé/lifetime privé, aucune API SDK ou règle OIDC modifiée. Son delta conserve les61régressions OIDC.

### Raccord HMAC-only / SDK

L'[annexe §10](../contracts/oauthconnect-oidc-service-boundary.md#10-amendement--appstate-hmac-only-sans-autorit%C3%A9-jwt) borne le champ d'extraction SDK obligatoire à un **JWKS inline vide** : pas d'autorité JWT, clé ou utilisateur par défaut, acquisition réseau ou refresh. Le service ne requiert **aucun secret JWT global ni JWKS IAM** pour remplir cet état. `AppConfig.auth` conserve son parsing SDK, mais n'active pas de Bearer ; seules les clés HMAC d'instance authentifient les routes v2. Les clés privées de l'IdP, les clés publiques de ses clients OIDC, les secretsOAuth/HMAC et le TLS restent séparés.

### Exécution et portée

Le point d'entrée est `oauth-connect-service` : loaderstrict `OAUTH_CONNECT`, sélection `RUN_ENV` et fichiers `config/<RUN_ENV>.toml` depuis le répertoire courant, puis loggingSDK et construction complète avant bind. Le témoin Windows a effectivement chargé `config/test.toml` avec `RUN_ENV=test`, sans voie spéciale dans la production. `/oauth-connect/health` et `/oauth-connect/ready` utilisent le support plateforme ; readiness signifie catalogue initialisé, **pas fournisseur live/IAM/bascule opérationnels**.

IAM, fournisseur live, E2E GitHub/GitLab externes et déploiement restent **non qualifiés/hors lot**. TLSentrantnominal, signalWindowsgracieux, connexion3smesurée, streamsanslongueur, RSS et énumération système de sockets ne sont pas fermés par ces tests. Ledger détaillé local : `.cursor/handoffs/20261107-oauthconnect-oidc-service.md`. Plan ciblé : `.cursor/qa-plans/20261107-oauthconnect-oidc-service.md`, en continuité de QA20261106 T4/G-4 etT6/G-6. Les diagrammes restent leurs photographies datées, non des états live.

## Publication et nettoyage vérifiés

Le code et les tests sont publiés dans `origin/master` au commit `29bd917f43d79e42c2461a248868f657c14303a7` ; référence distante recoupée. Tous les livrables durables des branches de travail sont intégrés. Sauvegardes récupérables des propositions conservées avant alignement/retrait, sans force ni perte de modifications étrangères.

Les contrôles Git, dossiers et inventaire natif OpenCode ne montrent plus que le checkout principal ; `master` est la seule branche locale, arbre propre au contrôle, SDK `93b890ff` inchangé. Un retrait partiel avait laissé un répertoire vide et une entrée native orpheline : après vérification de vacuité, la registration Git a été reconstituée au commit publié, puis retirée normalement par l'outil natif ; absence physique et disparition des deux inventaires vérifiées. Aucun prune/reset/force ni modification directe de registre.

Aucun processus Cargo/rustc ou binaire de test/service de la mission ne reste observé. Aucun Docker/Kind/WSL n'a été créé ou démarré ; pas de nettoyage de ressources étrangères. Le serveur canvas partagé est conservé. Cette clôture concerne le lot OIDC/service local décrit ci-dessus, pas une bascule fournisseur ou de déploiement.
