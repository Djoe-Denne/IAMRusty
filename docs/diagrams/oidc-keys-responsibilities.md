# OIDC — clés, rotation et responsabilités

**Photographie du 7 novembre 2026.** Explication de la cible ADR0413
**Accepted / Partial** et du code effectivement lu, pas une nouvelle décision,
une procédure de rotation autorisée ou une qualification de déploiement live.

> **Archive historique, avant la fin du lot service Connect et le raccord IAM.**
> Les descriptions de composition refusante et de nonce IAM hors lot sont datées.
> Voir l'[atlas courant du8novembre](iam-oauthconnect-current.md), son
> [HTML autonome](iam-oauthconnect-current.html) et le [sommaire](README.md).

## Réponse courte : stockées où, tournées par qui ?

OAuthConnect est un **client OIDC / Relying Party**, pas l’émetteur des ID tokens.
La clé privée RS256 reste chez le **fournisseur OIDC** (GitLab ou autre autorité) :
son opérateur signe les ID tokens et publie les clés **publiques** via le `jwks_uri`
annoncé par Discovery. Connect ne génère, ne stocke et ne renouvelle pas cette clé
privée. Le profil OAuth GitHub utilise les API JSON, pas un JWKS d’identité.

- **GitLab.com** : la gestion et la rotation côté fournisseur relèvent de
  l’opérateur SaaS, pas de notre service.
- **GitLab self-managed** : elles relèvent des Ops de cette instance, selon sa
  version et son mécanisme officiel. La version et le déploiement fournisseur
  concrets n’ont pas été inspectés ici.
- **Emplacement privé réel : inconnu à confirmer avec Ops.** Le protocole et la
  documentation GitLab OIDC consultée ne prouvent ni un chemin de fichier, ni une
  base de données, ni un HSM pour notre fournisseur. Aucun de ces stockages n’est
  supposé. Les clés des **ID tokens de jobs CI** ne sont pas une preuve sur les
  clés du login OIDC ; ne pas appliquer une procédure CI à ce parcours.

## Schéma et consultation hors ligne

- [HTML autonome](oidc-keys-responsibilities.html) : deux vues SVG (frontières des
  clés, puis rotation et snapshot), fiches des modules ; CSS intégré, sans JS,
  React, CDN, police distante ni requête externe.
- Provenance : Canvas original local au poste de création, sans dépendance de
  consultation ni lien absolu propre à une machine. Le précédent
  [schéma d’architecture](oauthconnect-architecture.html) est préservé.
- Ouvrir l’HTML par double-clic ; pour un PDF : **Ctrl+P → Enregistrer en PDF → A4
  paysage**, désactiver les en-têtes/pieds du navigateur, ajuster l’échelle au
  besoin. Les deux vues restent vectorielles. Aucun PDF/JPEG n’est généré ici.
- Le rendu visuel et la pagination PDF ne sont **pas vérifiés** : navigateur MCP
  déconnecté, sans tentative répétée. Les contrôles structurels sont distincts.

## Ce que stocke réellement le code Connect

Le cache se trouve dans `services/OAuthConnect/infra/src/oidc/cache.rs` :

- `KeyCache` conserve un `Mutex<State>` et une garde async single-flight distincte
  (`:24–35`). `State.keys` et les snapshots sont des `Arc<CoreJsonWebKeySet>`.
  `OidcClient` partage son `Instance` par `Arc` (`oidc.rs:108–124`). Le partage est
  celui des clones du **même client d’instance**, pas un cache global par issuer
  ou `kid` ; les processus/pods indépendants ne partagent pas cet état.
- C’est un **cache technique de clés publiques en mémoire**, sans fichier de
  cache, Redis, DB ou persistance OpenBao ; un redémarrage le perd et recharge le
  JWKS initial. Ni sticky-session, ni état de corrélation authorize→complete.
- `OidcClient::new_with_clocks` valide puis acquiert Discovery et **un** JWKS
  initial par le callback borné avant de retourner un client (`oidc.rs:88–125`).
  Échec initial = constructeur refusé. L’exigence **avant écoute** appartient à
  la composition cible ; `build_clients`/`Application` sont encore refusants.
- Issuer et URLs découvertes sont attachés au client ; un refresh lit le JWKS
  épinglé, sans redécouverte ni URL `jku/x5u` choisie depuis un token.
- `Instance::protocol` reconstruit le client standard à partir des métadonnées
  validées avec **le snapshot retenu** (`oidc/protocol.rs:63–77`), pour que le
  verifier et UserInfo signé n’utilisent pas éternellement les clés du boot.

### Bornes et échecs, pas calendrier de rotation

`cache.rs:14–15,38–141` fixe et applique les bornes suivantes :

- Cache frais si âge depuis le **dernier succès < 300 s** ; à **300 s** il est
  expiré. Le TTL n’est pas prolongé par un échec de refresh.
- Tentative suivante seulement si **≥ 30 s** depuis la précédente tentative,
  succès **ou échec**. L’acquisition initiale consomme aussi cette fenêtre :
  un nouveau `kid` trop tôt après le boot peut être refusé.
- Une seule requête GET de refresh en vol par client d’instance/processus ; les
  autres requêtes recontrôlent génération et résultat. Annulation : la garde est
  libérée, sans tâche périodique/détachée ni flag persistant bloqué.
- Clé connue + cache frais : pas de GET. Seul `NoMatchingKey` peut demander un
  refresh borné, suivi d’une nouvelle vérification du **même ID token**
  (`oidc.rs:248–295`). Autre erreur de signature, ambiguïté, algorithme ou claim
  invalide : refus, pas refresh systématique. Pas de nouvel échange POST du code.
- Expiration + refresh impossible ou en échec : **fail closed**, sans clé périmée
  de secours ni désactivation des vérifications. Après un échec pour un nouveau
  `kid`, une ancienne clé **encore fraîche** peut néanmoins valider un autre token
  signé par elle. La révocation d’une clé publique n’est donc pas instantanée.
- Plusieurs replicas ont des horloges/fenêtres et snapshots indépendants : pas
  de rate limiter distribué ni garantie de refresh simultané. Une rotation qui
  réutilise le même `kid` n’est pas promise avant l’expiration du TTL.

**300 s ne signifie pas “rotation automatique toutes les cinq minutes”.**
Connect actualise des clés publiques à la demande ; il ne pilote pas le signer.

## Coordination d’une rotation avec le fournisseur

OIDC Core §10.1.1 décrit la publication de nouvelles clés au `jwks_uri`, le `kid`
du token et la conservation temporaire des anciennes clés publiques. Pour cette
cible, coordonner avec l’opérateur fournisseur :

1. Publier la nouvelle clé publique **avant** de l’utiliser pour signer avec un
   nouveau `kid`, avec une avance compatible avec les caches et leurs refus bornés.
2. Conserver l’ancienne clé publique pendant la validité des ID tokens encore
   acceptables **et les marges convenues** ; la durée exacte dépend du fournisseur
   et de ses paramètres. Aucun nombre ni calendrier de rotation n’est inventé ici.
3. Vérifier la procédure officielle, les durées des tokens, le chevauchement des
   clés, TLS/CA, disponibilité du JWKS et comportement des replicas avant bascule.
   Aucun test live de cette coordination n’a été exécuté.

Le cooldown peut refuser un nouveau `kid` ; une panne JWKS peut faire refuser des
logins. **Pas de garantie zéro downtime**, y compris après restart. Ne pas utiliser
une boucle de redémarrages pour contourner les bornes.

Changer les clés sous le **même issuer** est une rotation ; remplacer l’issuer
sous le **même slug** est un changement d’autorité, interdit opérationnellement.
Cela exige une nouvelle autorité/slug et un cadrage de linking, pas un cache reset.

## Responsabilité de chaque module

Chemins ci-dessous relatifs à `services/OAuthConnect/`, sauf mention contraire.
Les numéros sont ceux du **canon lu le 07/11/2026**, pas ceux du worktree docs b55.

- **`configuration/src/validation.rs`** — valide catalogue, profils, secrets et
  scopes ; `validate_oidc:220–237` impose PKCE, `openid`, issuer et origines admises.
  Déclare la confiance, ne va pas chercher une clé privée fournisseur.
- **`setup/src/secrets.rs::resolve_secrets:26–69`** — résout les `client_secret`
  et HMAC au boot via SDK/OpenBao KV v2 ou mode direct explicite, sans résultat
  partiel ni fallback implicite. Ce n’est pas le stockage JWKS.
- **`setup/src/transport.rs::build_http_client:17–52`** — HTTP de confiance injecté,
  vérification TLS par défaut, CA explicite si configurée, sans redirection/retry
  automatique ; deadlines et bornes de lecture restent distinctes.
- **`setup/src/mixed.rs::build_clients:27–36` / `app.rs::Application:20–50`** —
  composition de tous les clients avant écoute et ownership du serveur en cible.
  **Tranche C encore refusante** au moment de la lecture, pas service opérationnel.
- **`infra/src/oidc/callback.rs::BoundHttp:20–143`** — garde chaque URL avant I/O,
  Discovery et JWKS interne inclus ; remet à la bibliothèque les octets originaux
  bornés. Le helper transport conserve le plafond décompressé, pas de parser JWT maison.
- **`infra/src/oidc/protocol.rs::Instance::protocol:63–77`** — client standard
  `openidconnect` 4.1, métadonnées validées + snapshot public retenu, méthode
  d’authentification du token endpoint. Pas d’upgrade protocolaire implicite.
- **`infra/src/oidc/cache.rs::KeyCache:32–141`** — snapshots mémoire,
  TTL/cooldown/single-flight et échecs/annulation ; aucune gestion de clé privée.
- **`infra/src/oidc/verify.rs::validated:13–72`** — signature RS256 et claims
  issuer, audience, exp/iat/nbf, azp, nonce ; `at_hash` contrôlé s’il est présent.
- **`infra/src/oidc/profile.rs::Profile` + `oidc.rs::completion:191–221`** —
  `sub` exact, projection minimale, UserInfo conditionnel et contrôle de son
  subject/conflits ; sortie tokens + profil normalisé, jamais ID token/claims bruts.
- **`http/src/lib.rs::create_router:17–39`** — registry et montage des routes v2
  par instance ; réemploie le récepteur `idp-connect-contract` et son HMAC sur
  PATH complet d’origine. Ce module n’est pas un serveur JWKS fournisseur.
- **IAM** — UUID, comptes, liens, transactions et règles métier. Le raccord v2 et
  le nonce durable restent hors lot ; ne pas présenter ce raccord comme déjà livré.

### Cinq matériaux cryptographiques distincts

- **RS256 du fournisseur** : privé chez l’émetteur, public JWKS lu par Connect,
  vérification de l’identité externe.
- **`client_secret` OAuth/OIDC** : authentifie le client au token endpoint ;
  résolu au démarrage, sans lien avec la clé privée RS256 de l’émetteur.
- **HMAC IAM→Connect** : secret symétrique par instance pour la requête S2S
  METHOD/PATH/TIMESTAMP/BODY ; pas une clé de signature d’ID token.
- **Clés JWT IAM plateforme** : indépendantes, pour les tokens émis par IAM.
  Le `auth.jwt.jwks_url` legacy pointant vers IAM ne désigne pas le JWKS GitLab.
- **TLS / CA** : certificat, clé serveur et racines de confiance transport ;
  ni un JWKS d’identité, ni le secret HMAC, ni le secret client OAuth.

## Est-ce déjà en accord avec les déploiements ?

**La cible est cadrée ; la bascule n’est pas livrée.** Inspection en lecture seule
des fichiers du dépôt, sans interroger un cluster, un conteneur ou un coffre :

- `docker-compose.yml:339–381` construit encore **GitHubConnect/GitLabConnect**,
  avec leurs configs `/app/config` et certificats `/app/certs`. Les commentaires
  et `services/IAMRusty/config/development.toml:40,50` conservent les URLs HTTP
  internes et préfixes `/github-connect`, `/gitlab-connect` ; pas les routes
  `/oauth-connect/{instance}/v2/...`. L’écart HTTP/HTTPS est déjà signalé par ADR0413
  (`:67`) ; ce n’est pas une preuve HTTPS bout en bout du runtime.
- `services/GitLabConnect/config/development.toml:29–31` utilise encore authorize,
  token GitLab.com et **`/api/v4/user`**, pas la validation OIDC nouvelle. Son
  `:18` pointe le JWKS **IAM** pour la config JWT plateforme, pas les clés OIDC
  fournisseur. Les secrets/PEM/tokens n’ont pas été lus ni reproduits.
- GitHub legacy déclare OpenBao et un token-file
  (`services/GitHubConnect/config/development.toml:26,37–54`). La seule stanza
  Compose inspectée monte config + certificats, sans mount token-file explicite :
  sa disponibilité effective est **à vérifier**, pas une panne affirmée ici.
- `ops/deploy/apps/overlays/local-full/oauth-proof/resources.yaml:1–33` déclare
  **un replica legacy GitLabConnect** en HTTPS, config et certificats référencés
  par Secrets montés en lecture seule. `oauth-proof/iam.toml:31` vise ce service ;
  `oauth-proof/gitlab.toml:23–25` sélectionne `isolated_test` et un **simulateur
  HTTP**, pas un fournisseur GitLab réel. Cela ne prouve ni une rotation RS256 ni
  la conformité d’un cluster live.
- Le gabarit `local-full/github-openbao/github.toml:17,24–38` sépare références
  client/HMAC, CA OpenBao et token-file ; `agent.hcl:4,19` décrit adresse HTTPS
  et destination. Ce sont des **déclarations**, pas des volumes/processus observés.
- `runtime/monolith/src/routes.rs:18–22` ne monte que IAM, Telegraph, Hive,
  Manifesto et Lazaret ; aucun nest OAuthConnect dans ce routeur inspecté.
  La cible conserve Connect hors processus IAM.
- La recherche exacte `OAuthConnect|oauth-connect-service|oauth-connect-http`
  dans `ops/deploy/**/*.{yaml,yml,toml}` n’a retourné aucune déclaration.
  Portée de cette recherche limitée aux fichiers suivis/visibles de ce répertoire ;
  elle n’exclut pas un déploiement externe et ne remplace pas une inspection live.

Restent : composition C, raccord IAM/nonce durable, configuration des instances et
secrets/CA/endpoints effectivement montés, bascule vers v2, preuve externe
`sub GitLab == ancien id`, E2E fournisseurs et stratégie de rollback. Conserver les
legacy pendant la transition, les UUID et les liens ; aucun déploiement promis.

## Preuves, sources et limites

- **Sources du code** : racine Serena canonique du dépôt vérifiée,
  code OIDC effectif au canon ; ce worktree
  documentaire est à la base `b55e0ce`, où les anciens stubs ne décrivent pas les
  modules OIDC nouveaux. Aucun `activate_project`, aucune édition de code.
- **Preuve communiquée par le parent** : gate ciblé de 60 tests et Clippy terminé
  exit0 après correction de deux fixtures MIME, pas d’assouplissement UserInfo
  production. Preuve non rejouée par cet auteur ; pas de qualification globale
  du service, de publication ni de déploiement déduite de ce résultat.
- [ADR0413](../adr/0413-oauthconnect-multi-instances-profils.md),
  [contrat multi-instances §§2.4,3–6](../contracts/oauthconnect-multi-instances.md),
  [annexe OIDC/service §§4,6,9](../contracts/oauthconnect-oidc-service-boundary.md),
  [frontière HTTP/secrets](../contracts/oauthconnect-http-secrets-boundary.md).
  Leurs photographies historiques T0 ne supplantent pas le code effectif lu.
- Ledger local canonique : `.cursor/handoffs/20261107-oauthconnect-oidc-service.md`,
  consulté au checkout canonique et non embarqué ici.
- Sources officielles lues le **07/11/2026** :
  [GitLab comme fournisseur OIDC](https://docs.gitlab.com/integration/openid_connect_provider/)
  (scope `openid`, Discovery GitLab.com/self-managed ; aucune preuve sur notre
  emplacement privé ni calendrier réel de rotation),
  [OIDC Core §10.1.1](https://openid.net/specs/openid-connect-core-1_0.html#RotateSigKeys)
  (`jwks_uri`, `kid`, renouvellement public et rétention des clés récemment retirées).
- **Inconnus Ops** : version/mode d’hébergement effectif de GitLab, magasin de clé
  privée, opérateur/procédure et calendrier exacts, durée/chevauchement des clés et
  tokens, endpoints/CA et montages réellement actifs. Aucun secret lu ni commande
  Docker/Kind/kubectl, aucun test applicatif, aucun runtime lancé par cet auteur.
