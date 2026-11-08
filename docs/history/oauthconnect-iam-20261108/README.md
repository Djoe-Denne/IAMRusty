# OAuthConnect / IAM — histoire de la session du 6 au 8 novembre 2026

Cette synthèse documente la conception, les tentatives, les corrections et les preuves rapportées pendant la refonte OAuthConnect puis son raccord à IAM. Elle conserve les étapes antérieures au raccord : contrat v2, configuration, mapping JSONPath, budget d'évaluation, transport, secrets, OIDC et service opérationnel.

**Il s'agit d'une histoire datée, pas du contrat actuel ni d'une preuve de déploiement.** ADR et contrats portent les décisions ; le code et les rapports de campagne attestent ce qui était réalisé à une révision donnée. Un projet de solution n'est pas automatiquement une décision Accepted. Une revue statique, une compilation et un test exécuté sont trois preuves différentes.

- [Répertoire des sessions et messages de provenance](sessions.md)
- [Inventaire machine et limites de couverture](coverage.json)
- [Guide actuel : clés, JWKS, rotation, HMAC et TLS](../../guides/iam-oauthconnect-securite-clefs-jwks-tls.md)
- [Sources techniques du guide](../../guides/iam-oauthconnect-securite-sources.md)

## 1. Périmètre, méthode et confidentialité

### Une frontière fixe, pas une collecte autoréférentielle

La session racine est `ses_e4db70249ffd7v3AV0y0zQ6443`, « OAuth Connect multi-instance architecture ». La frontière inclusive est le **8 novembre 2026 à 12:25:19.929 UTC**, message `msg_1bb1ecac5001mUOtb937gHb5pE` : l'utilisateur demande de capitaliser l'intégralité de la session et des sous-agents, de conserver les schémas appréciés et de détailler JWKS, rotation des clés, responsabilités et TLS. Les réponses et feuilles créées ensuite pour cette nouvelle mission documentaire sont exclues. [Provenance racine](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

L'inventaire natif accessible comporte **39 sessions réellement liées** : la racine, **36 anciennes feuilles**, **2 nouvelles feuilles exclues**. Les anciennes feuilles déclarent toutes cette racine comme parent ; aucun petit-enfant ni `fork` n'est déclaré dans le corpus lié accessible. Dix événements de compaction sont des messages, pas dix sessions supplémentaires. [Inventaire](coverage.json)

Les **37 captures anciennes** contiennent **4 488 messages observés**, dont **4 458 avant frontière** et **30 messages racine ultérieurs exclus**, sur **234 pages épuisées**. Zéro doublon d'ID et aucune ancienne session absente ne sont observés dans la capture finale. Les messages des deux nouvelles feuilles ne sont pas lus ni comptés. Les métadonnées du projet ont été paginées jusqu'au terme : 503 sessions, dont 464 sans lien avec cette racine non retenues ; un recoupement séparé des archives et enfants directs n'a révélé aucun ancien enfant supplémentaire. [Détail des comptes](coverage.json)

### Ce qui est analysé, ce qui n'est pas publié

La source est l'API native OpenCode V2 en lecture seule, notamment `GET /api/session` et `GET /api/session/{sessionID}/message`. Les affiliations sont établies par `parentID` et, lorsqu'il existe, `fork.sessionID`, jamais par similarité de titre. Le schéma consulté est la [description OpenAPI officielle](https://opencode.ai/docs/openapi.json).

Les réponses natives sont filtrées en mémoire avant toute écriture d'archive. Le présent dossier ne publie **ni transcript, ni projection privée, ni entrée d'outil, ni raisonnement interne, ni message système/développeur, ni résumé de compaction, ni credential ou clé privée**. Il contient seulement une synthèse et des métadonnées de traçabilité. Les identifiants de messages permettent de retrouver une preuve dans la source native autorisée ; ils ne constituent pas un lien de partage de son contenu.

La projection privée initiale retient les textes publics user/assistant, les statuts et une sélection stricte de résultats d'outils. Elle recense **159 messages user**, **570 parties assistant de type text**, **4 759 appels d'outils** et exclut **5 542 parties natives reasoning** sans les résumer. **512 messages restent limités à leurs métadonnées** : 33 système, 210 synthétiques, 227 idle, 29 changements de lieu, 1 changement d'agent, 2 changements de modèle et 10 compactions. Leur contenu n'est pas analysé ; des détails de consentement/runtime peuvent donc ne pas figurer dans la surface publique. Ces 512 sont inclus dans les 4 458 : ne pas les ajouter au total. [Politique de couverture](coverage.json)

Les résultats d'outils privés sont réduits de 18 043 132 caractères observés à 52 736 caractères de statuts/résultats sélectionnés ; les échos de scripts sont écartés. Les 57 085 070 octets de réponses sources deviennent 4 630 187 octets de projections privées expurgées, soit environ 91,9 % de réduction, **pas une compression sans perte**. Les hashes publiés dans le manifeste servent uniquement à l'intégrité de ces projections privées : aucun lien vers un fichier temporaire absent n'est proposé. [Mesures et exclusions](coverage.json)

**Aucune exhaustivité absolue n'est revendiquée** : sessions supprimées, liens de fork non déclarés, contenus volontairement privés et modifications concurrentes d'une session active restent des limites. Un outcome natif `succeeded` d'agent n'est jamais interprété comme un test passant.

## 2. Le besoin métier et les choix initiaux — 6 novembre

### Éviter un service par fournisseur

La demande de départ est de remplacer la duplication GitHubConnect/GitLabConnect par un **OAuthConnect multi-instances** qui serve une liste de fournisseurs configurés. Le nombre futur de fournisseurs justifie de chercher une description déclarative plutôt que d'ajouter systématiquement du code par fournisseur. L'utilisateur demande aussi d'évaluer les crates disponibles. [Demandes initiales et évaluation](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

La distinction retenue est **profil OIDC standard** versus **profil OAuth avec acquisition de ressources JSON et projection**. L'idée utilisateur décrit chaque claim par une API et un JSONPath, en commençant par l'identifiant stable et l'email. Pour OIDC, un nom standard reste invariant lorsqu'il est présent ; cela ne signifie pas que chaque fournisseur fournisse obligatoirement email, email vérifié et preferred username. [Profils, noms et champs optionnels](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

La compatibilité avec IAM est examinée avant de coder. Une identité externe n'est pas un UUID de compte interne. Une instance logique ou un issuer n'est pas un serveur physique : plusieurs replicas ou clusters ne doivent pas créer artificiellement plusieurs identités. L'utilisateur fait préciser la différence entre email nécessaire à un parcours d'authentification et profil utilisable pour relier un compte déjà authentifié. [Questions IAM et portée d'instance](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

Le futur **on-behalf-of** est une limitation explicite à documenter : le fournisseur sert aujourd'hui d'autorité d'identité ; les futures actions GitHub/GitLab au nom de l'utilisateur ne découlent pas automatiquement de cette refonte. Les E2E réels GitHub et GitLab sont prévus comme un travail distinct. [Limitation et E2E séparés](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

### TDD et contrat v2

Le 6 novembre à 20:08 UTC, la demande d'implémentation déclenche architecte et QA planner en parallèle. Le travail de cadrage métier est considéré déjà fait ; aucune orchestration supplémentaire n'est présentée comme un livrable fonctionnel. [Déclenchement](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

Le socle du contrat v2 apporte deux routes POST explicites, HMAC avant extraction JSON, nonce canonique de 43 caractères représentant 32 octets, validation nonce/PKCE avant I/O et aucune composition/fallback legacy implicite. Son auteur livre un candidat sans l'exécuter : le parent doit intégrer et valider. [Livraison du socle](sessions.md#ses_e4d65cf4dffe50fPpPtQM7F2z4)

La revue identifie un **RED comportemental réel** : 13 tests découverts, 5 PASS et 8 FAIL, sur des assertions de statuts 404 contre 200/401/400/422, pas sur une compilation cassée. [Qualification RED](sessions.md#ses_e4d55b055ffebmDOYj90nDXYt4) La suite évolue ensuite à 16 tests ; une correction Clippy conserve leurs attentes et compteurs. [Témoins conservés](sessions.md#ses_e4d5ff69dffeHUsT9L7q3x7cL5) Le PASS sécurité est explicitement statique et borné : il ne signifie pas GREEN exécuté. [Revue sécurité](sessions.md#ses_e4d518c08ffeb6PmFj7FUA8axK)

Sources de décision : [ADR 0413](../../adr/0413-oauthconnect-multi-instances-profils.md), [contrat multi-instances](../../contracts/oauthconnect-multi-instances.md).

## 3. Configuration, projection JSONPath et budget — 7 novembre

Cette tranche est séparée du transport, d'OIDC et d'IAM. Les oracles couvrent grammaire et politique AST : les refus de fonctions ou de descente récursive ne reposent pas sur un simple filtre textuel de JSONPath. La revue de 41 cas distingue **18 caractérisations déjà vertes** et **23 signaux RED attendus** ; les expressions négatives parsent réellement dans la version pinnée avant d'être refusées par la politique. [Revue des oracles](sessions.md#ses_e4abcc502ffe9dqNmMAhcWdDhN)

Un diagnostic précis évite de corriger le mauvais problème : `$.id trailing` est interprété par `jsonpath-rust` comme un nom contenant un espace, via le whitespace implicite Pest ; ce n'est pas un suffixe simplement ignoré après parsing. Le correctif vise cette permissivité établie. [Diagnostic parser](sessions.md#ses_e4abd5fbbffe0kShdSQtoWBAlP)

Le finding sécurité **S-1** impose une admission conservatrice **avant l'évaluation exhaustive**, un cumul global du budget et une liaison de l'admission aux entrées originales immuables. L'auteur livre les estimations/préflights. [Candidat budget](sessions.md#ses_e4abd5fbbffe0kShdSQtoWBAlP) La revue ciblée constate la mitigation, sous réserve des unités privées et gates restant au parent. [Recoupement sécurité](sessions.md#ses_e4ab10ef8ffeh7Hf5ANNikHwbZ)

Le parent annonce ensuite **114 tests verts**, fmt/check/Clippy strict réussis, publication **`3ba3e3c`**, cinq worktrees retirés et aucun runtime créé. À cet instant, transport, secrets résolus, OIDC et IAM restent à faire ; les E2E restent externes. [Clôture de la tranche](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

Sources : [contrat multi-instances](../../contracts/oauthconnect-multi-instances.md), [budget JSONPath](../../contracts/oauthconnect-jsonpath-budget.md), [rapport configuration/projection](../../missions/20261107-oauthconnect-next.md).

## 4. Transport OAuth, secrets et composition — 7 novembre

La tranche suivante distingue le client HTTP de confiance construit au setup, la résolution de secrets SDK/OpenBao et le vrai client composé. Les oracles sont relus avant implémentation : exactitude redirect/PKCE/nonce, POST token, lecture bornée, secrets et builder. [Revue des témoins](sessions.md#ses_e4a392995ffeJMgQh2DVrRJNgd)

Le parent rapporte les onze tests C du vrai client composé : délai de 10 secondes, TLS CA/SAN, gzip, HMAC et loader isolé. [Résultat parent](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443) La revue sécurité C clôt T2 à partir de cette preuve et de son oracle source ; elle refuse d'en déduire une qualification OIDC/IAM/E2E. [Portée du PASS C](sessions.md#ses_e4a2be888ffeY3V0HEWgSmFNHD)

Le lot est annoncé clos avec **189 tests verts**, fmt/check/Clippy réussis, **`origin/master=eabc645`**, cinq worktrees retirés et aucun runtime créé. Les limites annoncées sont conservées : **connexion 3 secondes et streaming sans longueur non mesurés** ; OIDC, service opérationnel, IAM et E2E encore hors tranche. [Bilan transport/secrets](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

À la question sur gzip, la réponse précise qu'il s'agit de **compatibilité HTTP, pas d'une exigence OAuth** : reqwest décompresse une réponse fournisseur puis le lecteur borne le contenu décompressé à 1 Mio avant JSON. Ce choix ne compresse ni codes, ni tokens, ni secrets. OpenBao garde la compression désactivée. Le parent reconnaît que cette compatibilité n'était pas indispensable au besoin initial. [Explication du choix gzip](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

Une revue HMAC est aussi rectifiée après comparaison des blobs : l'assertion `pair.zero()` existait bien dans le commit RED ; le premier artefact untracked relu n'était pas le blob committé. Le finding T-S1 est retiré. Le diff de cet état untracked ancien reste non reproductible : la contradiction est expliquée, pas effacée. [Rectification de revue](sessions.md#ses_e497d49e5ffeb7gW8lcp2En4yh)

Sources : [frontière HTTP/secrets](../../contracts/oauthconnect-http-secrets-boundary.md), [rapport transport/secrets](../../missions/20261107-oauthconnect-transport-secrets.md).

## 5. OIDC, JWKS et service opérationnel — 7 novembre

### 5.1. Les clés n'ont pas toutes le même propriétaire

La demande du 7 novembre à 16:09 UTC porte expressément sur stockage des clés OIDC, rotation et responsabilités de chaque module. [Demande et explication publique](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

- **Fournisseur OIDC** : la clé privée RS256 signe les ID tokens ; l'opérateur du fournisseur organise sa rotation. Le stockage concret du GitLab utilisé — fichier, base, HSM — n'est pas établi par la session.
- **OAuthConnect** : seules les clés publiques obtenues par Discovery et `jwks_uri` sont conservées en cache RAM, par instance/processus/replica. Ce cache n'est pas un coffre de clés privées.
- **OpenBao ou configuration explicite** : secrets OAuth `client_secret` et HMAC interservices, distincts du JWKS public, des clés JWT IAM et des certificats TLS.
- **IAM** : comptes, UUID et liens ; ses propres responsabilités JWT ne doivent pas être attribuées au JWKS du fournisseur.

[Schéma des responsabilités et limites](sessions.md#ses_e49cbab1bffeaqRcz0N7HaOmXC)

### 5.2. Snapshot, TTL, cooldown et rotation

Le candidat OIDC décrit un cache partagé par clones d'une instance, fraîcheur **strictement inférieure à 300 secondes**, cooldown **au moins 30 secondes**, tentative enregistrée avant I/O, single-flight, recontrôle de génération/résultat et annulation sans tâche détachée ni reset opportuniste du cooldown. Le refresh en cas d'absence de clé est lié à `NoMatchingKey`, pas à tout échec de signature. Le client standard est reconstruit à partir du **snapshot JWKS réellement retenu**, par `metadata.set_jwks`. [Handoff du candidat](sessions.md#ses_e4abd5fbbffe0kShdSQtoWBAlP)

L'explication à l'utilisateur expose la séquence opératoire habituelle : publication de la nouvelle clé publique/nouveau `kid`, début de signature, maintien temporaire de l'ancienne clé publique pour les tokens encore valides. Cette séquence n'est **pas une observation d'un fournisseur réel**. Les **300 secondes sont un TTL de cache**, jamais une fréquence de rotation ; redémarrer Connect ne renouvelle aucune clé fournisseur. Après expiration avec refresh en échec, l'authentification est refusée. [Explication rotation/cache](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

### 5.3. Deux rouges UserInfo signé : un défaut de fixture MIME

Le premier passage annoncé est **58/60**, avec deux échecs UserInfo signé dont un après rotation. [Résultat intermédiaire](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443) Le diagnostic établit que WireMock 0.6.5 réécrit `Content-Type` en `text/plain` après `set_body_string`, malgré un header ajouté ensuite. `openidconnect` sélectionne légitimement JSON/JWT selon le MIME. La correction proposée est `set_body_raw(..., "application/jwt")` dans les fixtures, sans changer signatures, claims ou assertions, sans assouplir la production. **Ces deux rouges ne démontrent pas un bug du snapshot JWKS.** [Cause et correction ciblée](sessions.md#ses_e4abd5fbbffe0kShdSQtoWBAlP)

Les quarante premiers oracles OIDC avaient été relus avec leurs versions de dépendances et leurs préflights. Le reviewer annonçait **static-presence**, pas leur exécution. [Revue des tests A](sessions.md#ses_e4970ed13ffeuXzBM9OSwBp2bk)

### 5.4. URLs Discovery et paramètres réservés

Le callback préinspecte les URLs du type standard `CoreProviderMetadata` avant le GET JWKS interne : `http::Uri` supprimerait sinon un fragment avant que la garde voie l'URL originale. Les octets ne sont pas resérialisés par une réécriture JSON. [Callback et piège de représentation](sessions.md#ses_e4abd5fbbffe0kShdSQtoWBAlP)

Le finding LOW OIDC-S1 ajoute le contrôle des **neuf noms réservés OAuth déjà existants**, sur noms de query décodés et avant JWKS ; une query non réservée reste permise. Le prédicat est partagé, pas dupliqué avec une politique divergente. [Extraction mécanique](sessions.md#ses_e491b82b6ffeIzEbWs6VJzS0eH) Le témoin couvre un nominal avec query autorisée et neuf noms réservés chacun littéral/percent-encodé. [Nouveau témoin](sessions.md#ses_e4ac5aeb9ffeBsKR3K4mO0tC3L) La revue clôt statiquement ce finding, sous réserve du GREEN exécuté par le parent. [Portée de la revue](sessions.md#ses_e493d161dffeEhTswdhuH1pWYJ)

### 5.5. Send, TLS et startup : trois causes distinctes

L'emergency-thinker résout deux blocages techniques : une contrainte de lifetime `'h: 'c` redondante dans `DiscoveryHttp`, puis une fixture TLS où CA et feuille partageaient les mêmes DN par défaut. La convention IAM de noms distincts est réemployée **sans désactiver CA ou SAN**. Son retour rapporte préflight TLS positif/négatifs et 61 régressions PASS, mais C setup encore **3 PASS / 8 RED T0** à cet instant. Ce résultat intermédiaire ne qualifie pas tout le service. [Secours Send/TLS](sessions.md#ses_e48f887e3ffeV1xGZ388Kkirw7)

Les quatorze premiers oracles C étaient relus comme **9 RED / 5 PASS attendus** contre une production T0 refusante ; ils n'étaient pas exécutés par leur reviewer. [Revue C](sessions.md#ses_e491163c9ffeW6qxu3LCGapug2)

Un autre blocage startup relève de la **dépendance à une configuration JWT plateforme inutilisée**, pas du TLS. L'architecte retient un extracteur SDK construit avec un JWKS vide, strictement refusant : aucune clé, source réseau, identité par défaut ni réutilisation HMAC. [Décision de composition §10](sessions.md#ses_e4d834425ffehYgxAsZJJl5CrU) Le parsing de la configuration `auth` reste ; l'API Connect est authentifiée par HMAC, **sans rôle d'authentificateur JWT**. [Microfix effectif](sessions.md#ses_e4ad0aa1effegB7v1FG79PA5BZ)

Des tests caractérisent ce refus face à des tokens HS256/RS256 valides dans leur propre montage. Ce sont des caractérisations post-correctif, pas un RED artificiel. [Témoins C16](sessions.md#ses_e4ac61246ffe3gDG9Fn6pXsVCy) La revue startup demeure statique et bornée ; le gap §10 est encore ouvert à son instant de retour. [Revue C/startup](sessions.md#ses_e48d210feffeBIcIVpJu1VLB6w)

### 5.6. Le bilan de la tranche, avant IAM

Le parent annonce **295 tests passants**, aucun échec/ignoré/filtré, fmt et Clippy strict sur sept paquets, revue sécurité sans finding restant, publication **`e8952d8`**, worktrees retirés et SDK inchangé. **À cette date, le raccord IAM, les E2E réels GitHub/GitLab et la bascule/déploiement restent hors lot.** Plans et certificats synthétiques ne prouvent aucune configuration TLS live. [Clôture OIDC/service](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

Sources : [frontière OIDC/service](../../contracts/oauthconnect-oidc-service-boundary.md), [rapport OIDC/service](../../missions/20261107-oauthconnect-oidc-service.md).

## 6. Raccord IAM v2 — 8 novembre

### 6.1. Un nonce durable, distinct du verifier PKCE

L'utilisateur demande ensuite de raccorder IAM. [Déclenchement](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443) Le scaffold prépare les types et les mappings avec des valeurs provisoires vides/absentes ; il ne doit pas être lu comme le comportement final. [Scaffold compilable](sessions.md#ses_e455d4b88ffe4jP4krG553x1lT)

Le domaine remplace ce provisoire par **32 octets OsRng**, tirage distinct du verifier, base64url sans padding, nonce persisté `Some` avant retour BEGIN **même sans PKCE**. Au consume, les gardes existantes restent ; la validation du nonce durable survient **après le retour du writer**, afin de ne pas empêcher la consommation/purge des lignes legacy ou corrompues. La capacité consommée permet une seule complétion v2 à partir des secrets originaux. [Domaine nonce/capacité](sessions.md#ses_e454d6265ffe5BXwXMyYY9DGIb)

Le writer SQL étend l'INSERT avec le nonce exact, récupère les deux secrets, applique la **purge atomique verifier + nonce**, conserve `FOR UPDATE`, prédicats et horloge, puis **commit avant retour**. Aucun filtre nonce-nonNULL ne bloque la consommation du legacy. La migration ajoute `TEXT NULL`, sans default/backfill/index ; `down` conserve la colonne et `up IF NOT EXISTS` permet la réapplication. Le handoff auteur est alors un candidat source, pas une preuve SQL exécutée. [Writer et migration](sessions.md#ses_e4544a2c3ffeHVpobs1A3pUPFc)

### 6.2. Migration LOGIN, LINK et RELINK sans réécrire les règles métier

Le démarrage transmet nonce/challenge/méthode exacts, jamais le verifier dans authorize. La complétion groupée tokens/profil remplace exchange+profile v1 : un seul appel, aucun retry/fallback. LOGIN conserve email requis avant lookup, identification, persistance et suites JWT/parcours incomplet. LINK/RELINK conservent les gardes provider/redirect/opération/cible. Le helper temporaire v1 n'est retiré qu'après migration du dernier appelant. [Livraisons successives domaine/LOGIN/LINK](sessions.md#ses_e454d6265ffe5BXwXMyYY9DGIb)

Le cadrage préfère **réexporter cinq types existants par `iam_domain::port::service`**, plutôt qu'ajouter DTO/helper ou dépendance application directe. La compilation des consommateurs prépare le lot : **ce n'est pas un RED comportemental**. [Amendement de couture](sessions.md#ses_e456930a8ffe5DkAMzIMbSixTP)

Les oracles LINK/RELINK distinguent six témoins exigeant le comportement v2 et huit caractérisations des gardes existantes ; les capacités sont obtenues par le chemin réel de consommation, pas fabriquées. [Témoins nonce/capacité et application](sessions.md#ses_e45560644ffevfOmn1HTkvVYyf)

Une revue indépendante révèle deux lacunes métier G7. [Audit des oracles](sessions.md#ses_e4517c25cffegyuJf741XREQmk) Les témoins ajoutés imposent :

- RELINK avec l'email primaire d'un autre utilisateur ne réaffecte aucun email ;
- LINK avec un nouvel email fournisseur l'ajoute **non vérifié**, même si le fournisseur le déclare vérifié ;
- une collision d'identité externe avec un autre compte refuse l'opération, conserve snapshots/liens/tokens, consomme/double-purge la transaction et refuse le replay sans nouvel appel complete.

Aucun garde métier ni statut précis nouveau n'est inventé pour rendre ces tests verts. [Témoins email/collision](sessions.md#ses_e453b21beffe4azeET8ucSMffQ)

### 6.3. Transport IAM, corps bornés et vérification HTTPS

Le connector v2 valide nonce/PKCE avant I/O, signe PATH complet et corps exact via le helper existant, ne retente pas en v1, lit les chunks avec plafond inclusif **1 Mio**, ne lit pas le corps non2xx et renvoie des erreurs génériques. Connexion 3 s ajoutée, total 10 s et redirects désactivés conservés ; sources reqwest verrouillées **0.11.27** vérifiées. Streaming sans longueur et gzip ne sont pas prouvés par ce seul auteur. [Livraison transport](sessions.md#ses_e45541a57ffe0qBtjsLLZWiF6w)

Les onze témoins natifs couvrent deux slugs/HMAC, nonce sans PKCE, refus avant I/O, limites exacte et +1 octet, redirections et timeout réel. [Candidats transport](sessions.md#ses_e455cc19affeYVyUoVmvxTeYz8) Les tests HTTPS conservent le nominal v1 et ajoutent nominal v2, mauvaise CA, mauvais SAN, redirection vers un second listener pourtant trusted sans forwarding. [Transposition HTTPS](sessions.md#ses_e454a4f5fffe9IjBHGk43oUPMh)

### 6.4. Revues, montage SQL, panne Windows et relance

Le plan QA isole non-rejeu/commit/double-purge, refus sans effets IAM, transport et régressions. [Plan proportionné](sessions.md#ses_e456895e9ffef0qzf10IpN3g63) Les revues sécurité et correctness sont **statiques et bornées**. [Sécurité](sessions.md#ses_e452b6716ffeTAqTunP21eTNX1) ; [correctness](sessions.md#ses_e45181526ffeSzYFWRbgMf3kMM) Le reviewer de tests précise qu'à son instant de revue les onze binaires IT n'ont pas encore tourné. [Limite de l'audit](sessions.md#ses_e4517c25cffegyuJf741XREQmk)

**Premier incident : prérequis SQL de l'upgrade.** Résultat 8 PASS / 1 FAIL PG23502 avant l'upgrade : un UUID `user_emails.id` manque dans le montage. Le microfix fournit cet UUID sans modifier les oracles migration/snapshots/contraintes. Ce rouge ne démontre pas un défaut comportemental de migration. [Diagnostic et microfix](sessions.md#ses_e4552a8ccffexnxfkjq7biFGoq) ; [sortie historique 8/1](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

**Browser transactions :** 17/17 passent, y compris email, collision RELINK, course entre callbacks et observation de la purge avant completion. [Résultat historique](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

**Second incident : démarrage opaque de fixture callback.** Un conteneur Postgres existe, mais la fixture n'a pas obtenu l'ID retourné à la création et le classe `UNKNOWN`. Les dix-sept échecs callback sont des échecs de montage. Les collisions suivantes avec le port déclaré `54070` sont **secondaires** : elles ne prouvent ni la cause initiale ni qu'un processus écoute sur ce port. [Incident et sorties rouges](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

Le mécanisme aléatoire existe, mais Windows choisit d'abord un port par bind `127.0.0.1:0`, ferme la socket, puis Docker reçoit ce port précis. La fenêtre entre sélection et utilisation n'est pas une réservation durable. C'est une **possibilité générale**, pas la cause établie de cette occurrence. Le harness jette l'erreur originale de testcontainers, ce qui explique le diagnostic opaque. Le précédent du 19 octobre, alors corrélé à une forte pression Docker, est cité comme même classe d'incident ; les protections déjà en place aujourd'hui empêchent de conclure à une cause identique. [Explication et limites causales](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

L'utilisateur demande de ne pas approfondir et de relancer. Le callback repasse **17/17 sans modification du code**. Le parent conserve séparément rouges et verts et maintient que la cause initiale reste inconnue. **La relance verte ne transforme pas une hypothèse de course de port en diagnostic prouvé.** [Demande et relance](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

### 6.5. Les comptes finaux et leur portée

Les sorties historiques d'unités se décomposent en **30 + 87 + 62 + 146 = 325**. Le bilan IT expose onze suites : **9 + 17 + 2 + 11 + 17 + 4 + 11 + 5 + 2 + 1 + 1 = 80**, sans failed/ignored/filtered :

- `auth_transactions_postgres` : 9 ; `oauth_browser_transactions` : 17 ;
- `oauth_cleanup_lifecycle` : 2 ; `auth_oauth_start` : 11 ;
- `auth_oauth_callback` : 17 ; `auth_oauth_provider_failures` : 4 ;
- `relink_provider` : 11 ; `verified_idp_https` : 5 ;
- `organization_account_guard` : 2 ; `app_cleanup_supervision` : 1 ;
- `request_trace_redaction` : 1.

Ces résultats peuvent être réaffichés dans un résumé de logs : on ne les additionne pas à nouveau comme des runs supplémentaires. [Messages des sorties et bilan](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

Le dernier bilan parent annonce **`origin/master=b732b60` vérifié**, **325 unités + 80 IT**, dix worktrees temporaires retirés et arbre propre. **Clippy infra reste rouge sur dette préexistante** ; E2E fournisseurs et cutover restent hors lot. Les 114/189/295 des tranches précédentes sont des périmètres intermédiaires/cumulatifs : **ne pas les ajouter aux 325+80**. Cette capitalisation documentaire n'a réexécuté aucun test. [Bilan final IAM](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

Sources : [contrat IAM v2](../../contracts/iam-oauthconnect-v2.md), [mission IAM](../../missions/20261108-iam-oauthconnect-v2.md), [résultats IAM](../../missions/20261108-iam-oauthconnect-v2-results.md).

### 6.6. Ressources retenues : aucune propriété inventée

Le parent déclare onze fixtures identifiées retirées et aucun conteneur possédé actif. Un conteneur jamais démarré et douze volumes anonymes ont une propriété insuffisamment tracée pour une suppression automatique ; ils ne sont pas adoptés par leur seul nom ou un événement Docker. Le dernier bilan public indique leur conservation comme demandé. La surface user publique ne contient pas nécessairement le détail d'une sélection de formulaire, les contenus synthétiques étant exclus. [Question de rétention et bilan](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

## 7. Les schémas appréciés : du rendu à la documentation durable

Trois demandes se complètent : un schéma Canvas pour comprendre l'implémentation ; sa sauvegarde en documentation visualisable HTML/PDF/image ; un schéma des responsabilités OIDC/JWKS. La dernière demande d'ingestion insiste sur l'explication approfondie JWKS, rotation et TLS. [Demandes de schémas et capitalisation](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

Les livrables historiques sont [architecture OAuthConnect](../../diagrams/oauthconnect-architecture.md) et son [HTML autonome](../../diagrams/oauthconnect-architecture.html), puis [clés et responsabilités OIDC](../../diagrams/oidc-keys-responsibilities.md) et son [HTML autonome](../../diagrams/oidc-keys-responsibilities.html). Le rendu Canvas local n'est pas à lui seul la source durable.

Pour le second schéma, l'agent rapporte un HTML UTF-8 autonome, sans JS/ressources externes, 60 primitives SVG, 39 libellés et 10 fiches identiques entre supports. **Rendu React/visuel et pagination PDF non vérifiés** : le navigateur était alors déconnecté. Aucun PDF/JPEG produit ni validation d'un déploiement réel n'est revendiqué. [Retour de l'auteur des schémas](sessions.md#ses_e49cbab1bffeaqRcz0N7HaOmXC)

Ces supports sont des photographies datées : l'explication du 7 novembre présente encore `setup::mixed/app` en construction et les manifests legacy en place. Il faut préserver cette date, puis distinguer la version actuelle après raccord IAM, sans faire passer l'ancien schéma pour preuve d'un cutover. Stockage privé réel GitLab, procédure active de rotation, certificats/CA TLS live restent à confirmer auprès des sources et opérateurs appropriés. [Photographie et inconnues opérationnelles](sessions.md#ses_e4db70249ffd7v3AV0y0zQ6443)

## 8. Lire l'histoire sans surinterpréter ses preuves

- **Review ≠ exécution** : un PASS statique peut être juste alors qu'un gap de test reste ouvert.
- **Préparation ≠ RED** : compilation, scaffold et montage de prérequis ne sont pas des rouges comportementaux.
- **Caractérisation déjà verte ≠ nouveau RED** : les refus existants et les invariants conservés sont utiles sans inventer une panne préalable.
- **Résultat partiel ≠ PASS global** : 58/60 OIDC ou 3 PASS / 8 RED C ne qualifient pas le service complet.
- **Succès final ≠ cause connue** : la relance callback verte ne démontre pas pourquoi le premier démarrage a échoué.
- **Code/fixtures ≠ production** : certificat synthétique, rotation simulée et test local ne prouvent ni fournisseur live ni bascule.
- **Inventaire accessible ≠ histoire absolue** : la pagination complète de l'état natif accessible n'inclut pas les contenus volontairement exclus, sessions supprimées ou liens non déclarés.

Le [répertoire](sessions.md) conserve les **36 auteurs/feuilles**, leurs rôles, interventions successives, comptes et identifiants de messages. Le [manifeste](coverage.json) distingue les 37 captures historiques des deux nouvelles feuilles exclues, et décrit exactement ce que les empreintes privées attestent — et ce qu'elles n'attestent pas.
