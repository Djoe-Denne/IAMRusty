# IAM / OAuthConnect — dossier de sources sur clés, JWKS et TLS

**Date de lecture : 8 novembre 2026.** Ce dossier rend vérifiable le
[guide pédagogique](iam-oauthconnect-securite-clefs-jwks-tls.md), sans audit global,
exécution de tests ni observation d'un environnement live.

<a id="perimetre"></a>
## 1. Base, méthode et portée

- Base Git de la lecture : **`eb32349be94a4211ee4e26ae64b88786a2df5604`**.
  HEAD du checkout canonique et du worktree documentaire confirmé identique au
  début de l'enquête. Les numéros ci-dessous sont **1-based à cette base**, pas
  des numéros supposés stables après édition. Les liens relatifs restent portables.
- Les corps Rust ont été lus via Serena, racine canonique AIForAll confirmée ;
  le worktree de documentation n'est pas l'index partagé. Aucun changement de
  projet, reconstruction d'index ou mutation du SDK. Les nouveaux fichiers docs
  sont ajoutés par patch uniquement dans le worktree autorisé.
- Les symboles connus ont été lus directement, puis les consommateurs nécessaires
  ont été suivis : cache→verifier, route HMAC→client, handler JWKS→usecase→registry,
  handler rotate→façade→rotator→provider. Pas d'inventaire global des dépendances.
- Les références Serena de `rotate_signing_scope` ont échoué une fois avec
  `content modified (-32801)` ; le réessai ciblé a réussi. Les résultats montrent
  le wrapper organisation, la méthode `rotate_scope` du rotator et le réexport
  infra. Ils ne prouvent pas l'absence de callers dynamiques ou d'outils externes.
- Une hypothèse de fichier `crates/idp-connect-contract/src/security.rs` a été
  refusée parce que ce fichier n'existe pas ; la localisation exacte a retrouvé
  `hmac.rs`. Ce n'est pas une panne Serena ni une absence de politique de sécurité.
- Les sorties longues et standards ont été traités via Context Mode. Les textes
  primaires ont été acquis puis leurs sections/extraits utiles effectivement lus,
  pas seulement les résumés d'un moteur. Les références normatives sont en §3.
- **Aucun** Cargo, DB, Docker, Kind, rotation, accès coffre live, test fournisseur,
  inspection TLS live, modification de configuration ou opération Git de mutation
  n'a été effectué pour cette enquête documentaire.

### Vocabulaire de preuve

- **Vérifié — source :** corps/configuration/contrat lus à la base indiquée ;
  n'implique pas une nouvelle validation runtime.
- **Rapporté — preuves antérieures :** résultat consigné dans un document de
  mission antérieur ; non réexécuté ici.
- **Prescription :** règle d'ADR/contrat, à ne pas transformer en constat de
  déploiement par le seul fait de sa rédaction.
- **Inconnu / non inspecté :** absence d'observation suffisante ; aucune valeur,
  équipe, version ou automatisation ne doit être inventée.

<a id="faits-locaux"></a>
## 2. Carte des faits sensibles

### S01 — Frontières et statut du lot

**Énoncé :** une instance Connect est une autorité logique configurée, non un
Pod ; deux profils OAuth/OIDC ; IAM garde comptes, UUID, transactions et métier.
API v2 authorize+complete, nonce indépendant IAM, ID token non exposé dans le
résultat ; cible globale **Accepted / Partial**, pas bascule.

- **Autorité :** [ADR0413](../adr/0413-oauthconnect-multi-instances-profils.md),
  lignes3–5,36–41,51–57,68–75 ;
  [multi-instances](../contracts/oauthconnect-multi-instances.md) ;
  [raccord IAM v2](../contracts/iam-oauthconnect-v2.md), §§1–7.
- **Code recoupé :** [OIDC completion](../../services/OAuthConnect/infra/src/oidc.rs),
  `OidcClient::completion`,191–221, retourne `ProviderAuthentication` avec
  tokens et profil ; pas d'ID token dans ce DTO de sortie.
- **Statut :** vérifié source/contrat. Les325unités+80IT et295tests sont rapportés,
  voir S18 ; pas de preuve E2E obtenue par cette lecture.
- **Norme associée :** [OIDC Core §2](https://openid.net/specs/openid-connect-core-1_0.html#IDToken),
  identité du client/RP et ID token. L'architecture multi-instances reste un choix local.

### S02 — Cache : valeurs et précédences exactes

**Énoncé :** TTL300s, cooldown30s, acquisition initiale dans la première fenêtre ;
fraîcheur strictement `<300s`, tentative admissible à `>=30s` ; échec n'actualise
pas la date de succès ; snapshot expiré jamais utilisé comme fallback.

- **Source :** [cache.rs](../../services/OAuthConnect/infra/src/oidc/cache.rs),
  constantes14–15 ; `KeyCache::new`38–51 ; `snapshot`53–69 ; `current`71–81 ;
  `refresh`83–121 ; `record_attempt`123–141.
- `current` demande le refresh **avant** la vérification si le snapshot est
  expiré. `record_attempt` enregistre avant l'I/O. Un refresh vide/non valide
  ne devient pas un succès et ne remplace pas le snapshot par un succès vide.
- **Statut :** vérifié — source. Constantes de cette implémentation, **pas des
  délais imposés par OIDC**, ni la fréquence de rotation du fournisseur.
- **Norme associée :** [Core §10.1.1](https://openid.net/specs/openid-connect-core-1_0.html#RotateSigKeys)
  explique le renouvellement à `kid` inconnu et le chevauchement ; ne fixe pas300/30.

### S03 — Partage, single-flight et annulation

- **Sources :** [cache.rs](../../services/OAuthConnect/infra/src/oidc/cache.rs),
  `Snapshot`17–22, `State`24–30, `KeyCache`32–36, `refresh`83–121 ;
  [oidc.rs](../../services/OAuthConnect/infra/src/oidc.rs),
  `OidcClient::new_with_clocks`85–125 et construction `Arc<Instance>`.
- **Énoncé :** clones du même client d'instance partagent les clés ; instances et
  processus distincts non fusionnés par `kid` ou issuer. Une garde async sérialise
  les refresh et les waiters recontrôlent génération/résultat. Annulation libère
  la garde ; pas de job périodique/détaché dans ce cache. La tentative déjà
  enregistrée peut néanmoins continuer à imposer le cooldown après annulation.
- **Statut :** vérifié — source, plus noms de témoins privés lus par overview
  dans [tests.rs](../../services/OAuthConnect/infra/src/oidc/tests.rs) :
  `g4_successful_refresh_is_single_flight_for_concurrent_distinct_codes`,
  `g4_failed_refresh_releases_all_waiters_without_herd_and_next_window_recovers`,
  `g4_cancelling_the_refresh_owner_releases_the_guard_for_a_future_window`.
  Ces témoins n'ont pas été exécutés ici et leur seul nom ne remplace pas le corps du cache.
- **Limite :** aucun rate limiter distribué ni synchronisme des replicas prouvé.

### S04 — Refresh seulement sur la bonne erreur ; ancienne clé fraîche

- **Source de décision :** [oidc.rs](../../services/OAuthConnect/infra/src/oidc.rs),
  `OidcClient::verified_identity`248–295. Seul le variant typé
  `SignatureVerificationError::NoMatchingKey` demande le refresh réactif ;
  seconde validation du même token avec nouveau snapshot, aucun deuxième POST code.
- **Snapshot effectif :** [protocol.rs](../../services/OAuthConnect/infra/src/oidc/protocol.rs),
  `Instance::protocol`63–77 reconstruit le client depuis `metadata.set_jwks`.
- **Témoins dont les corps ont été lus :**
  [tests.rs](../../services/OAuthConnect/infra/src/oidc/tests.rs),
  `g4_failed_unknown_kid_refresh_preserves_fresh_old_key_but_never_renews_success_age`,
 232–248 ;
  `g4_same_kid_crypto_failure_does_not_refresh_early_but_new_snapshot_is_used_at_ttl`,
 250–284. Le premier refuse le nouveau `kid`, accepte ensuite l'ancienne clé
  encore fraîche puis refuse à expiration ; le second refuse le même `kid` avec
  nouveau matériau avantTTL, sans GET anticipé, puis acquiert àTTL.
- **Statut :** vérifié — corps sources/tests, pas nouvelle exécution.
- **Conséquences :** ni refresh sur tout échec signature/claim/algorithme, ni
  acceptation stale illimitée, ni révocation instantanée, ni garantie zéro downtime.
- **Norme :** [JWT BCP §3.1](https://www.rfc-editor.org/rfc/rfc8725.html#section-3.1),
  restriction d'algorithme ; [Core §10.1.1](https://openid.net/specs/openid-connect-core-1_0.html#RotateSigKeys),
  rôle du `kid`, sans promesse locale sur sa réutilisation.

### S05 — Discovery, issuer et URLs épinglées

- **Sources :** [oidc.rs](../../services/OAuthConnect/infra/src/oidc.rs),
  `new_with_clocks`85–125 ;
  [callback.rs](../../services/OAuthConnect/infra/src/oidc/callback.rs),
  `BoundHttp::{new,guard,guard_endpoints,discover,send}`27–143 ;
  [protocol.rs](../../services/OAuthConnect/infra/src/oidc/protocol.rs),
  `validate_metadata`80–111.
- **Énoncé :** issuer configuré exact, Discovery/JWKS initial via callback borné,
  contrôle des origines avant I/O et des endpoints découverts. Refresh au
  `metadata.jwks_uri()` fixé, pas nouvelle Discovery. Aucune URL `jku/x5u`
  reçue du token ne commande le GET du chemin cache inspecté.
- **Métadonnées :** JWKS initial non vide, RS256 annoncé, Code flow, auth token
  compatible ; ces contraintes ne signifient pas support universel OIDC.
- **Statut :** vérifié — source ; aucune observation d'un issuer fournisseur réel.
- **Norme :** [Discovery §4.3](https://openid.net/specs/openid-connect-discovery-1_0.html#ProviderConfigurationValidation),
  égalité issuer Discovery/configuration/ID token et abandon sur échec.
- **Autorité inchangée :** modifier l'issuer sous un slug n'est pas une rotation,
  règle de [0413](../adr/0413-oauthconnect-multi-instances-profils.md) et du
  [contrat multi-instances](../contracts/oauthconnect-multi-instances.md).

### S06 — Vérification cryptographique et claims

- **Source :** [verify.rs](../../services/OAuthConnect/infra/src/oidc/verify.rs),
  `validated`13–72 : RS256 seul, autres audiences refusées, UTC injecté,
  `iat <= now+60s`, bibliothèque `claims(verifier, nonce)`, `exp > now`,
  `iat <= exp`, `azp` présent égal client_id, `nbf` présent non futur ; `at_hash`
  présent calculé par bibliothèque sur access token et clé/algorithme effectifs.
- **Source complémentaire :** [oidc.rs](../../services/OAuthConnect/infra/src/oidc.rs),
  `completion`191–221 : UserInfo conditionnel si email absent, lié au subject
  validé, merge avant sortie du résultat complet.
- **Statut :** vérifié — source. Le détail de projection/conflits reste régi par
  `oidc/profile.rs` et le contrat ; pas qualification de claims réels fournisseur.
- **Normes :** [Core §3.1.3.7](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation),
  [Core §5.3.2](https://openid.net/specs/openid-connect-core-1_0.html#UserInfoResponse),
  [RFC8725 §§3.1,3.3,3.8–3.12](https://www.rfc-editor.org/rfc/rfc8725.html#section-3).
  Les60s et le refus des audiences supplémentaires sont des choix locaux.

### S07 — Secrets de démarrage : KV/direct, HMAC et secret OAuth différents

- **Sources :** [secrets.rs](../../services/OAuthConnect/setup/src/secrets.rs),
  `resolve_secrets`26–69, `bundle`99–116 ;
  [mixed.rs](../../services/OAuthConnect/setup/src/mixed.rs), `build_clients`30–66.
- **Énoncé :** validation complète, préparation direct avant acquisition, un
  resolver SDK OpenBao KVv2 si nécessaire, résolution des bundles sans résultat
  partiel. Secret OAuth préservé exactement ; HMAC trim puis minimum16octets,
  transféré tel quel dans `HmacKey`. Les clients conservent le snapshot au boot,
  pas de consultation coffre métier à chaque complete/JWKS refresh.
- **Statut :** vérifié — source de composition/résolution. La localisation du
  privé fournisseur, la politique coffre live et son détenteur restent inconnus.
- **Frontière :** KVv2 de credentials Connect ≠ Transit de signature privée IAM.
  [0311](../adr/0311-crypto-privee-deleguee-aux-providers.md) ;
  [frontière HTTP/secrets](../contracts/oauthconnect-http-secrets-boundary.md).
- **Norme associée :** [OAuth2 §2.3.1](https://www.rfc-editor.org/rfc/rfc6749.html#section-2.3.1),
  authentification client, distincte de la clé signant l'ID token dans le profil local.

### S08 — HMAC v2, PATH complet et fenêtre temporelle

- **Sources :** [server.rs](../../crates/idp-connect-contract/src/server.rs),
  `hmac_middleware`71–111 ; [hmac.rs](../../crates/idp-connect-contract/src/hmac.rs),
  `canonical_string`, `sign`, `verify`107–136 et constante `MAX_SKEW_SECS` ;
  [http_connector.rs](../../services/IAMRusty/infra/src/auth/http_connector.rs),
  `signed_post`129–164 ;
  [router Connect](../../services/OAuthConnect/http/src/lib.rs), `create_router`17–39.
- **Énoncé :** secret propre à l'instance, METHOD/PATH OriginalUri complet/
  TIMESTAMP/BODY, comparaison constante, refus si écart absolu `>30s`,64KiB
  maximum entrant au middleware avant parsing/client. Pas fallback v1 des parcours v2.
- **Statut :** vérifié — source. Le PATH porte l'instance ; le corps ne choisit
  pas une autre clé. Aucune garantie de rejet de tout replay dans la fenêtre
  ne se déduit du seul MAC temporel.
- **Autorité :** [0409](../adr/0409-confiance-callback-oauth-idp-connect.md),
  [0413](../adr/0413-oauthconnect-multi-instances-profils.md),
  [raccord v2 §6](../contracts/iam-oauthconnect-v2.md#6-adapter-http-iam--v2-bornes-et-expurgation).
  Protocole local, pas un mécanisme standard JWKS.

### S09 — AppState HMAC-only, pas autorité JWT Connect

- **Sources :** [app.rs](../../services/OAuthConnect/setup/src/app.rs),
  `Application::new`36–69, `hmac_only_user_id_extractor`137–141 ;
  [router](../../services/OAuthConnect/http/src/lib.rs), `create_router`17–39.
- **Énoncé :** extracteur SDK construit avec exactement `{"keys":[]}` et aucune
  audience ; état de support HTTP ne peut pas fournir de clé de confiance JWT.
  Routes montées via `router_v2`, HMAC indépendant par `ConnectState`.
- **Canon explicite :** [annexe OIDC/service §10](../contracts/oauthconnect-oidc-service-boundary.md#10-amendement--appstate-hmac-only-sans-autorité-jwt),
  usage refusant local d'une API SDK documentée pour les tests, pas approbation
  générale de ce helper comme authentificateur de production.
- **Statut :** vérifié — composition locale + contrat. Le SDK au pin93b890ff
  n'est pas réaudité en totalité et n'a pas été modifié. Aucun JWT Connect ni
  JWKS fournisseur inséré dans l'état, aucun Bearer utilisateur implicite.

### S10 — Transaction IAM et consommation avant complete

- **Sources :** [oauth_transaction.rs](../../services/IAMRusty/domain/src/entity/oauth_transaction.rs),
  `ConsumedOAuthTransaction::complete_authorization`149–176 ;
  [writer](../../services/IAMRusty/infra/src/repository/oauth_transaction_write.rs),
  `consume`70–110 ;
  [oauth_service.rs](../../services/IAMRusty/domain/src/service/oauth_service.rs),
  `process_callback`102–165.
- **Énoncé :** SELECT primaire avec tous bindings et `FOR UPDATE`, UPDATE
  consumed_at + deux secretsNULL, commit avant retour. Complétion depuis redirect,
  verifier et nonce originaux de la capacité. LOGIN appelle complete puis contrôle
  email avant lookup/métier et sauvegarde tokens ensuite.
- **Nonce et legacy :** détails de génération32octets, persistance avant authorize
  et refus legacy après consommation régis par [raccord v2 §§3–4](../contracts/iam-oauthconnect-v2.md),
  recoupés par les preuves antérieures S18 ; pas relecture de tous les tests nonce ici.
- **Statut :** source ciblée + prescription + résultats rapportés, pas test DB.
- **Norme associée :** [BCP OAuth §2.1.1](https://www.rfc-editor.org/rfc/rfc9700.html#section-2.1.1),
  valeurs transactionnelles liées au client et user-agent ; nonce explicite local
 0413, pas obligation indistincte de tout Code flow.

### S11 — JWT IAM : délégation privée, public et binding d'émission

- **Canon :** [0311](../adr/0311-crypto-privee-deleguee-aux-providers.md),16–27 ;
  [0304](../adr/0304-jwt-acces-plateforme-rs256-jwks.md), décision et réalité courante.
- **Source :** [setup app.rs](../../services/IAMRusty/setup/src/app.rs),
  `resolve_access_signing`1140–1206 : Transit d'abord s'il est configuré, sinon
  remote configuré, sinon chemin local soumis à ses conditions. Pas de fallback
  local après erreur du provider configuré ; les `JwtKeyPair` délégués ont un privé vide.
- **Émission :** [jwt_encoder.rs](../../services/IAMRusty/infra/src/token/jwt_encoder.rs),
  `require_active_signing_epoch`207–253, `confirm_emission`255–269,
  `encode_rs256`392–433. Public provider comparé au public enregistré,
  signature du binding courant, vérification candidate puis fence writer finale.
- **Statut :** vérifié — corps. Ne prouve pas coffre/credentials/provider choisis
  live, ni possession d'une matrice cloud complète, ni révocation des JWT déjà émis.
- **Norme associée :** [RFC8725 §3.3](https://www.rfc-editor.org/rfc/rfc8725.html#section-3.3),
  valider les opérations cryptographiques ; la fence d'époque est une règle locale.

### S12 — Rotation IAM : qui appelle, qui fabrique, qui active ?

- **Demande d'administration réelle dans le code Hive :**
  [router Hive](../../services/Hive/http/src/lib.rs),
  `with_organization_signer_routes`115–142 : route rotate authentifiée,
  permissionAdmin sur `organization` ;
  [handler](../../services/Hive/http/src/handlers/organization_signer.rs),
  `rotate_organization_signer`89–110, principal vers commande ;
  [commande](../../services/Hive/application/src/command/organization_signer.rs),
  handler `RotateOrganizationSignerCommand`317–329, appel port puis persistance statut ;
  [client HTTP IAM](../../services/Hive/infra/src/iam/organization_signer_client.rs),
  `rotate_organization_signer`172–181, POST chemin IAM interne.
  Ce suivi établit le chemin produit Admin→Hive→IAM, pas l'identité d'un
  opérateur live ni un PASS de la politique OpenFGA/credentials.
- **Déclencheur HTTP inspecté :** [organization_signer.rs HTTP](../../services/IAMRusty/http/src/handlers/organization_signer.rs),
  `rotate_organization_signer`294–311, token de service interne, puis façade ;
  [façade application](../../services/IAMRusty/application/src/usecase/organization_signer.rs),
  `rotate`233–236 ; [setup](../../services/IAMRusty/setup/src/app.rs),
  `setup_signer_route_context`1105–1138, injection du rotator.
- **Orchestration :** [rotate.rs](../../services/IAMRusty/infra/src/signing/rotate.rs),
  `rotate_signing_scope`85–98, `insert_pending_rotation`100–192,
  `promote_pending_rotation`194–210.
- **Fabrication :** `rotate_provider_material`212–267 et `mint_transit_material`269–306 :
  seule branche Transit supportée pour la rotation livrée inspectée ; nouvelle
  version récupérée et épinglée, public obtenu, preuve avant admission writer.
  RemoteHttp/cloud retournent `ProviderNotSupported`, PEM ne génère pas de successeur.
- **Activation :** [registry](../../services/IAMRusty/infra/src/repository/signing_key_registry.rs),
  `promote_signing_key`2153–2249 : contrôle révision/binding/ancien Active,
  nouveauActive + ancienRetiring + publication commités atomiquement.
- **Statut :** vérifié — source. Prépublication durable réelle, mais aucun délai
  de diffusion imposé aux consommateurs dans l'enchaînement du helper.
- **Limites :** route organisation ≠ route plateforme démontrée ; primitive scope
  et tests plateforme ne prouvent pas un endpoint plateforme public/live.
  L'opérateur et l'ordonnancement réels sont inconnus. Le constructeur
  `start_background_tasks` dans [app.rs](../../services/IAMRusty/setup/src/app.rs),
 197–216, lance outbox et cleanup OAuth, pas un cron rotation dans cette fonction.
- **Signing leases :** le codec lu établit une fence finale d'époque, pas un
  service autonome de lease distribué. Ne pas attribuer au système une garantie
  de gel du signer/provider pendant tous les awaits ou de révocation rétroactive.
- **Norme associée :** [Core §10.1.1](https://openid.net/specs/openid-connect-core-1_0.html#RotateSigKeys)
  pour l'explication générique des publics ; le lifecycle IAM relève de0304/0311.

### S13 — Retrait IAM : TTL propre et publication matérialisée

- **Source :** [signing_key.rs](../../services/IAMRusty/domain/src/entity/signing_key.rs),
  `JWKS_RETIRE_SKEW_SECONDS`674 =60 ; `filter_jwks_publication_keys_at`691–716 :
  Pending/Active publiables, Retiring si updated_at non futur et
  updated_at+expiration_seconds+60s strictement supérieur à as_of ; Revoked exclu.
- **Config injectée :** [setup](../../services/IAMRusty/setup/src/app.rs),
  `setup_signer_route_context`1134–1135 et construction registry avec
  `config.jwt.expiration_seconds`. Pas de valeur universelle900s ni300s déduite.
- **Source :** [registry](../../services/IAMRusty/infra/src/repository/signing_key_registry.rs),
  `jwks_publication_snapshot`2314–2375, SELECT payload+horlogeDB primaire,
  reconstruction sous verrou si nécessaire avec politique TTL réelle.
- **Statut :** vérifié — source ; aucune observation de la valeur effective
  d'un déploiement. Retrait public ≠ suppression privée ni révocation instantanée
  dans tous les caches consommateurs.
- **Norme associée :** [Core §10.1.1](https://openid.net/specs/openid-connect-core-1_0.html#RotateSigKeys)
  prescrit un chevauchement raisonnable, pas ce nombre local.

### S14 — Publisher IAM, publication vide et consommateurs

- **Chemin réel :** [handler auth.rs](../../services/IAMRusty/http/src/handlers/auth.rs),
  `jwks`962–989 → `GetJwksCommand` →
  [usecase token.rs](../../services/IAMRusty/application/src/usecase/token.rs),
  `get_jwks`172–189 → registry S13. Vérifie révision et TTL de publication.
- **Énoncé :** publication writer valide vide autoritative ; erreur writer ou
  corruption ne devient pas succès vide ; pas de bootstrap réactivé par le publisher.
- **Distinction vérifiée :** la méthode synchrone
  [JwtTokenEncoder::jwks](../../services/IAMRusty/infra/src/token/jwt_encoder.rs),
 580–596, existe mais n'est pas la source de la publication HTTP suivie ici.
- **Prescription consommateur :** [0304](../adr/0304-jwt-acces-plateforme-rs256-jwks.md),37,
  renvoie à0308, âge maximal autoritatif60s, polls60s staging/prod et2s local/tests.
  Ce dossier ne qualifie pas chaque implémentation/proxy ni ces valeurs live.
- **Statut :** source publisher vérifiée ; politique consommateur citée comme
  prescription, sans l'assimiler aux300s/30s Connect.

### S15 — TLS vendor : default vérifié, exceptions explicites, CA additionnelle

- **Sources :** [configuration lib.rs](../../services/OAuthConnect/configuration/src/lib.rs),
  `VendorTransportSecurity`28–39 ;
  [setup transport.rs](../../services/OAuthConnect/setup/src/transport.rs),
  `build_http_client`17–52.
- **Énoncé :** Verified par défaut ; `LocalInsecure`/`IsolatedTest` explicites,
  pas inférés du build. Builder sans proxy ambiant, redirect ou retry automatique,
  gzip activé, autres décompressions désactivées ; CA PEM additionnelle par client.
  Les modes non Verified règlent `danger_accept_invalid_certs/hostnames(true)` ;
  ne pas les présenter comme simple ajout d'une CA privée de confiance.
- **Garde URL :** S05, HTTPS ou HTTP uniquement si exception déclarée, origine admise.
- **Statut :** vérifié — source. Pas de cert/version TLS/provider live observé.
- **Normes :** [RFC9525 §6](https://www.rfc-editor.org/rfc/rfc9525.html#section-6),
  [RFC5280 §6](https://www.rfc-editor.org/rfc/rfc5280.html#section-6),
  [BCP TLS §3.1.1](https://www.rfc-editor.org/rfc/rfc9325.html#section-3.1.1).

### S16 — TLS IAM → Connect : CA privée ne désactive pas le hostname

- **Source :** [http_connector.rs](../../services/IAMRusty/infra/src/auth/http_connector.rs),
  `with_security_mode_and_roots`77–126 : validation de l'URL par mode, clésHMAC
  validées, racines ajoutées au client, connect3s/total10s, redirects désactivés ;
  aucun `danger_accept_invalid_*` dans ce constructeur.
- **Énoncé :** Verified exige HTTPS, même avec racine additionnelle. Une exception
  d'URL HTTP n'est pas une désactivation des vérifications des certificats HTTPS.
  Ne pas recopier ici le comportement nonVerified du builder vendor Connect.
- **Statut :** vérifié — source ; tests CA/SAN rapportés S18, non rejoués ici.
- **Norme :** [RFC9525 §§6.1,6.3](https://www.rfc-editor.org/rfc/rfc9525.html#section-6.1),
  identité de référence déterminée depuis l'usage/configuration et comparaison
  avec l'identité présentée, pas acceptation de n'importe quel nom fourni.

### S17 — TLS réseau : cible, exemples et live ne se confondent pas

- **Autorité :** [0413](../adr/0413-oauthconnect-multi-instances-profils.md),74,
  signale explicitement la contradiction historique Compose HTTP interne versus0409
  HTTPS par défaut. L'ADR ne qualifie pas l'existant comme TLS de bout en bout.
- **Source documentaire ciblée :** [contrat exact-route Envoy](../contracts/0412-oauth-exact-route-envoy-proof.md),
  lignes40–57 et matrice locale : entrées de preuve locales HTTP et hops internes
  TLS/mTLS à distinguer. Ce n'est pas une inspection des manifestes actuellement
  chargés ni une lecture du certificat présenté par un terminator live.
- **Normes :** [RFC8446 §§2,4.4.3–4.4.4](https://www.rfc-editor.org/rfc/rfc8446.html#section-2)
  pour l'explication handshake, pas une preuve d'emploi TLS1.3 dans le dépôt.
- **Inconnus :** terminator et certificats live, CA réelle, cert-manager/ACME,
  reload, CRL/OCSP, mTLS effectif de chaque hop, propriétaire Ops et rollout CA.
  Aucune extrapolation depuis un exemple/manifeste/fixture à la production.

### S18 — Résultats locaux antérieurs et gaps

- **Connect :** [mission OIDC/service](../missions/20261107-oauthconnect-oidc-service.md)
  et [0413,36–41](../adr/0413-oauthconnect-multi-instances-profils.md),295tests
  rapportés sur fixtures ; non réexécutés par l'auteur du guide.
- **IAM :** [rapport final](../missions/20261108-iam-oauthconnect-v2-results.md),
  lignes5–7,13–25,29–37,44–53 :325unités+80IT retenues, SQL réel et TLSCA/SAN
  locaux ; aucun ignoré/filtré dans ces suites finales rapportées. Clippy strict
  infra reste rouge sur dette documentée ; ne pas remplacer par un GREEN global.
- **G5 :** absence de preuve intégrée IAM→Connect réel→vendor signé, faute couture
  existante dans le périmètre ; mocks S2S IAM ≠ preuve cryptographique OIDC intégrée.
- **Autres gaps :** fournisseurs réels, équivalence `sub` GitLab, cutover, retrait
  legacy ; limites dynamiques streaming/décompression/RSS consignées au rapport.
- **Statut :** résultats antérieurs rapportés, base documentaire du présent
  dossier eb32349 ; aucune nouvelle campagne ou preuve runtime.

<a id="standards"></a>
## 3. Sources primaires externes effectivement consultées

Les textes normatifs historiques restent des références actives pour leur objet ;
les BCP plus récents complètent les spécifications de base. Date de consultation
de cette enquête : **2026-11-08**. Les extraits pertinents ont été lus après
acquisition des pages, sans reprendre de JWT, secret ou clé privée de leurs exemples.

### N01 — OpenID Connect Core1.0, errata set2

- [§2 ID Token](https://openid.net/specs/openid-connect-core-1_0.html#IDToken) :
  rôle de l'ID token, `iss/sub/aud/exp/iat` et nonce.
- [§3.1.3.7 validation](https://openid.net/specs/openid-connect-core-1_0.html#IDTokenValidation) :
  issuer exact, audience du client, signature, temps et nonce lorsqu'envoyé.
- [§5.3.2 UserInfo](https://openid.net/specs/openid-connect-core-1_0.html#UserInfoResponse) :
  subject à lier à l'ID token. Le guide ne prétend pas qualifier tous les formats
  possibles de UserInfo sur un fournisseur réel.
- [§10.1 signature](https://openid.net/specs/openid-connect-core-1_0.html#Signing) et
  [§10.1.1 rotation](https://openid.net/specs/openid-connect-core-1_0.html#RotateSigKeys) :
  privé chez signataire, public auJWKS, nouveaukid et conservation des anciens publics.
  La section sur la rotation de **chiffrement**10.2.1 a été distinguée de10.1.1 :
  ne pas appliquer son déclenchement au signer d'ID tokens.

### N02 — OpenID Connect Discovery1.0, errata set2

- [§3 métadonnées](https://openid.net/specs/openid-connect-discovery-1_0.html#ProviderMetadata)
  et [§4.3 validation](https://openid.net/specs/openid-connect-discovery-1_0.html#ProviderConfigurationValidation) :
  issuer et `jwks_uri`, égalité de l'issuer et abandon sur échec de validation.
- Limite : découvrir un endpoint ne donne pas permission d'ignorer TLS, la
  garde d'origine, la restriction d'algorithmes ou les règles du client local.

### N03 — RFC7517, JSON Web Key

- [§4 paramètres](https://www.rfc-editor.org/rfc/rfc7517.html#section-4) :
  `kty/use/key_ops/alg/kid`, rôle de sélection dukid, `x5c` optionnel.
- [§5 ensemble](https://www.rfc-editor.org/rfc/rfc7517.html#section-5) : tableau `keys`.
- Limite : le format général sait représenter aussi du matériel secret ; le
  JWKS public de vérification ne doit pas le publier. `x5c` n'est pas synonyme
  du certificat TLS du serveur hébergeant le document.

### N04 — RFC7515, JSON Web Signature

- [§2 terminologie](https://www.rfc-editor.org/rfc/rfc7515.html#section-2) et
  [§3 présentation](https://www.rfc-editor.org/rfc/rfc7515.html#section-3) :
  signature/MAC, payload et encodage base64url, distincts du chiffrement.
- Limite : ce texte définit une enveloppe, pas les claims métier admissibles
  d'un access token IAM ou d'un ID token externe.

### N05 — RFC8725 / BCP225, bonnes pratiques JWT

- [§3.1](https://www.rfc-editor.org/rfc/rfc8725.html#section-3.1) : ensemble d'algorithmes
  choisi par l'application, pas acceptation libre de l'en-tête reçu.
- [§3.3](https://www.rfc-editor.org/rfc/rfc8725.html#section-3.3) : toutes les
  opérations cryptographiques doivent être validées.
- [§§3.8–3.12](https://www.rfc-editor.org/rfc/rfc8725.html#section-3.8) :
  issuer/subject/audience, données non fiables, typage et règles mutuellement
  exclusives entre sortes de JWT. La signature seule ne prouve pas le bon usage.

### N06 — RFC6749, OAuth2 de base, complété par RFC9700

- [§1.4](https://www.rfc-editor.org/rfc/rfc6749.html#section-1.4) : access token
  de ressource, opaque ou structuré selon le serveur.
- [§1.5](https://www.rfc-editor.org/rfc/rfc6749.html#section-1.5) : refresh token
  optionnel, destiné au serveur d'autorisation et non aux resource servers.
- [§2.3.1](https://www.rfc-editor.org/rfc/rfc6749.html#section-2.3.1) :
  authentification client avec mot de passe, distincte de la signature du fournisseur.

### N07 — RFC9700 / BCP240, sécurité OAuth, janvier2025

- [§2.1.1](https://www.rfc-editor.org/rfc/rfc9700.html#section-2.1.1) : protection
  du Code flow, PKCE, cas nonce OIDC et liaison transactionnelle client/user-agent.
- [§2.2.2](https://www.rfc-editor.org/rfc/rfc9700.html#section-2.2.2) et
  [§4.14](https://www.rfc-editor.org/rfc/rfc9700.html#section-4.14) :
  protection des refresh tokens ; ne pas assimiler leur rotation à celle des clés RSA.
- [§4.10.1](https://www.rfc-editor.org/rfc/rfc9700.html#section-4.10.1) :
  token lié à une preuve de possession, sujet distinct d'un Bearer simplement
  transporté par TLS. Aucune activation de DPoP/mTLS bound tokens revendiquée ici.

### N08 — RFC8446, TLS1.3

- [§2](https://www.rfc-editor.org/rfc/rfc8446.html#section-2) : déroulé handshake,
  échange de clés et séparation trafic handshake/application.
- [§4.4.3](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.4.3) :
  CertificateVerify démontre possession du privé et lie la preuve au handshake.
- [§4.4.4](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.4.4) : Finished.
- Limite : le guide explique le handshake complet à certificat, pas une mesure
  de versions négociées, reprisePSK,0-RTT,ECH ou mTLS du déploiement.

### N09 — RFC5280, validation X.509

- [§6](https://www.rfc-editor.org/rfc/rfc5280.html#section-6) : validation de
  chemin, date, chaîne et trust anchor comme entrée du vérificateur.
- Limite : exposer cette chaîne ne rend pas sa racine fiable par auto-déclaration ;
  aucune politiqueOCSP/CRL réelle des clients n'a été observée.

### N10 — RFC9525, identité de service TLS, novembre2023

- [§1.5](https://www.rfc-editor.org/rfc/rfc9525.html#section-1.5) :
  SAN/DNS-ID/IP-ID et terminologie des identifiants.
- [§6.1](https://www.rfc-editor.org/rfc/rfc9525.html#section-6.1) :
  identité de référence construite depuis l'usage/configuration du client.
- [§6.3](https://www.rfc-editor.org/rfc/rfc9525.html#section-6.3) : comparaison
  des nomsDNS. RemplaceRFC6125 ; ne pas présenter le seulCN comme substitut SAN.

### N11 — RFC9325 / BCP195, recommandations TLS/DTLS

- [§3.1.1](https://www.rfc-editor.org/rfc/rfc9325.html#section-3.1.1) :
  exclusions deSSLv2/v3 etTLS1.0/1.1 ; cadre contemporain de lecture des versions.
- Limite : documenter TLS1.3 pédagogiquement n'autorise pas à affirmer
  « TLS1.3-only partout » sans configuration et négociation observées.

<a id="inconnus"></a>
## 4. Inconnus à conserver explicitement

1. **Fournisseur OIDC réel :** version, type SaaS/autogéré effectif, stockage
   privé, HSM éventuel, rôle habilité, calendrier, overlap et procédure d'urgence.
2. **Signature IAM live :** provider et versions actifs, comptes techniques,
   coffre et chemins réels, opérateur, délégation, fréquence ou ordonnanceur externe.
3. **TLS live :** terminator par hop, certificats/SAN, CA, version négociée,
   mTLS, revocation policy, mécanisme de reload et responsable cert-manager/ACME.
4. **Propagation/revocation :** pas de mesure multi-replicas, délai live,
   garantie zéro interruption, introspection ou push de révocation démontrés ici.
5. **G5 et fournisseur réel :** absence de nouvelle preuve intégrée ; ne pas
   déclarer clos sur la somme de suites indépendantes.
6. **Exploitation de nouveaux providers :** remote Sign ≠ remote Rotate ;
   enum cloud ≠ support livré/qualifié. Pas d'extension du périmètre en audit P0–P6.

Ces inconnus n'empêchent pas d'expliquer les responsabilités de protocole. Ils
empêchent d'attribuer un propriétaire réel, un chemin privé ou un PASS runtime
sans observation et autorisation correspondantes.

<a id="ecarts-historiques"></a>
## 5. Écarts des notices et schémas du7novembre à réconcilier par le parent

Les originaux sont des photographies à préserver. Aucun de ces fichiers ni
leurs HTML/Canvas n'a été modifié par l'auteur des deux guides.

- [Architecture historique](../diagrams/oauthconnect-architecture.md),33–38 :
  annonce189tests, OIDC/service en cours et raccord IAM hors lot. **Évolution
  factuelle à présenter dans une nouvelle photographie datée :** service295
  et raccord325+80 livrés localement, toujours0413Partial, pas cutover/G5/live.
- [Clés/responsabilités historique](../diagrams/oidc-keys-responsibilities.md),
 54–55,126–128 : `build_clients`/Application encore refusants. **Source courante :**
  construction mixte et Application effectives, catalogue entier avant écoute.
- Même notice,145–146 : raccord v2 et nonce durable encore hors lot. **Source
  courante :** consommation double-purge et orchestration v2 livrées, sans
  réinterpréter l'ancienne phrase comme défaut à réimplémenter.
- Les liens absolus de source Canvas locale ne constituent pas une source
  portable pour un autre poste. Les Mermaid de ce guide sont des sources
  pédagogiques versionnées ; ils n'écrasent pas les SVG appréciés.
- **À conserver car toujours juste :** privé fournisseur non reçu par Connect,
  JWKS public mémoire local, TTL/cooldown bornés, same-kid sans refresh garanti,
  issuer différent≠rotation, GitHub OAuth API≠OIDC, localisation privée inconnue.
- **À compléter dans les schémas courants :** cycle IAM propre Pending/Active/
  Retiring/Revoked, TTL IAM+60 et fence d'émission ; cinq familles séparées ;
  frontièresTLS par hop ; AppState Connect inlinevide ne créant aucune autoritéJWT.
- Ne pas introduire de nouveaux chiffres de disponibilité ni de garanties
  cryptographiques globales, ni promouvoir0505Proposed enAccepted par réemploi.

<a id="maintenance"></a>
## 6. Maintenance et validation documentaire

Lors d'une mise à jour, relire les corps qui portent les constantes ou décisions,
suivre le consommateur réel et rattacher la photographie à un nouveau commit.
Changer un chiffre dans un tableau de bord sans relire le code ne suffit pas.

- Les liens du guide restent relatifs à `docs/guides/`, vers contrats, ADR et code.
- Les ancres explicites `idee-centrale`, `primitives`, `tokens`, `familles`,
  `parcours`, `validation`, `cache-connect`, `rotation-oidc`, `rotation-iam`,
  `tls`, `diagnostic`, `checklist`, `glossaire` servent aux futurs renvois.
- Les Mermaid représentent flux logique et responsabilités ; leur présence dans
  Markdown ne constitue pas une capture réseau ou un nouveau test runtime.
- Aucun JWT, token, code, nonce, verifier, secret client/HMAC, privé ni transcript
  de session réel ne doit être ajouté pour rendre les exemples plus concrets.
  `{"keys":[]}` est une absence de matériau, pas un secret sentinelle.
- Résultats de ce travail : **analyse statique et rédaction**, non PASS runtime.
  La publication, les diagrammes courants, le wiki et le ledger relèvent du parent.

### Contrôles documentaires effectués sur les deux nouveaux fichiers

- 96 liens relatifs : toutes les cibles existent ; 17 ancres Markdown citées
  retrouvées par contrôle statique des ancres explicites/titres, sans navigateur.
- 31 fichiers Rust liés comparés par empreinte entre canon et worktree : identiques.
  Cette comparaison confirme la version des sources citées, pas la couverture
  exhaustive du système ou une réussite de tests.
- Six blocs Mermaid, douze délimiteurs de code équilibrés, aucun espace de fin de
  ligne lors du contrôle. **Pas de parseur/rendu Mermaid exécuté**, ni impression
  PDF, ni rendu des HTML historiques dans ce lot ; validation visuelle au parent.
- Recherche ciblée de forme JWT compacte réelle et d'en-tête de clé privée :
  aucune correspondance ; revue rédactionnelle sans payload/credential réel.
  Ce contrôle de forme n'est pas une preuve universelle d'absence de secret.
- Inventaire Git du worktree : seulement ces deux nouveaux fichiers non suivis,
  HEAD eb32349 inchangé au contrôle. Aucun commit, index Git ou autre fichier modifié.
