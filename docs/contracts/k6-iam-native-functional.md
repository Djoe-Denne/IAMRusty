# Contrat implementer — k6 IAM password, préparation native bornée

- **Proposed**, 2026-11-11 ; GO utilisateur **« iam go »** pour écrire les tests k6 IAM et leur setup Windows natif. Base fournie : `b8a6b1e` ; implémentation/publication au parent. Ce fichier n'atteste aucun code ou run livré.
- **Périmètre partiel désormais visé : lecture et refus avant effets métier, sans compte créé ni SQL.** Le parcours positif à deux identités est **différé, pas abandonné** : inverse causal démontré ou accord utilisateur explicite de rétention à discuter, jamais présumé.
- Référence : [cadrage et option B retenue](k6-native-campaign-preparation.md). Les contrats Accepted/ADR-0504 restent inchangés ; nouvelle voie Windows ciblée, pas qualification rétroactive Linux ou obligation de porter les anciens JS.
- Impact **API-contrat d'outillage / cross-file**, limité aux scénarios k6 IAM, scripts natifs associés et recettes just. **Aucun Rust, autre service, CI, nouveau framework ou test unitaire de harness imposé.**

## 1. Cible et interface

Interface cible **non annoncée livrée** : `just k6-prepare iam`, `just k6-run iam`, `just k6-clean iam`. Un contexte privé de campagne explicitement désigné est commun aux trois commandes : run_id neuf, chemins privés, cible et effets possédés ; **aucune identité A/B préparée ou créée dans la tranche active**. Pas de profil arbitraire, plugin, Python d'autorité ou serveur de contrôle.

Cible fermée : `aiforall-local-full` / contexte `kind-aiforall-local-full`, IAM via le trajet mesh HTTPS loopback (`18443` dans le parcours opérateur). Préflight natif : Engine local Linux, labels/topologie et IDs des trois nœuds, contexte/namespace/UID des ressources utiles, route et IAM attendus ; noms/URL seuls insuffisants. **Forward propre à la campagne autorisé comme adaptation opérationnelle** : processus kubectl possédé, adresse `127.0.0.1`, port local éphémère observé, vers le **Pod Envoy vérifié par UID/chaîne de propriétaires**, port10000 correspondant au service attendu. L'origine devient `https://127.0.0.1:<port-possédé>/iam`, avec les mêmes exigences TLS/SAN ; pas d'URL libre ni adoption/arrêt d'un forward préexistant du keepalive. Aucun issuer/callback modifié, aucun OAuth. Aucun create/reset/build/start/rotation ou réparation implicite. Le dernier état rapporté est **API Kubernetes indisponible, port absent, keepalive trois nœuds** : pas un état fraîchement interrogé ici, ni autorisation de le réparer.

Outils Windows **déjà épinglés et présents**, chemins explicites ; pas de téléchargement, installation/activation de venv ou dépendance Python. Scripts PowerShell et CLI natifs seulement ; SQL/OpenSSL/Bao/FGA ne sont utilisés que pour un prérequis démontré, pas par principe.

**TLS bloquant :** sur Windows, la vérification Go `systemVerify` s'appuie sur le **trust store Windows** ; `SSL_CERT_FILE` n'est pas un raccord acquis. Prérequis : CA pertinente **déjà approuvée** et confiance effective de k6 au pin choisi ; aucun import CA global automatique. Vérifier chaîne/SAN par un **vrai smoke k6 HTTPS `/iam/health`** avant les autres cas IAM, jamais `-k`/`insecureSkipTLSVerify`, fallback HTTP ou proxy permissif. Les preuves curl/Schannel ne qualifient pas k6 ; absence de confiance = prérequis bloqué, pas modification implicite du magasin. La [documentation TLS k6](https://grafana.com/docs/k6/latest/using-k6/protocols/ssl-tls/) décrit TLS en général, pas une preuve de ce raccord Windows/pin.

## 2. Parcours concret : tranche active et cible différée

### 2.1 Tranche active sans création de comptes

Après le smoke TLS réel, cas bornés : **health**, **JWKS**, `GET /api/auth/username/check`, DTO signup invalides, complétion avec jeton invalide, refresh inconnu et `/api/me` sans Bearer, Bearer malformé ou headers de principal forgés. Réutiliser les vrais DTO/formats et fixer l'attendu précis de chaque route ; un refus gateway générique ne prouve pas la validation DTO IAM. Aucun compte/token valide préparé, login positif, rotation/replay d'un refresh valide ou isolation A/B ne sont exécutés.

Les POST sont exclusivement des **refus attendus avant écriture**, pas une preuve absolue d'absence d'effet si un défaut produit les accepte. Une acceptation inattendue fait échouer le cas et suspend la suite ; ne pas la masquer par un nettoyage SQL ou d'autres services. Résultats frais, complétude et cleanup restent distincts ; cette tranche partielle n'est pas un PASS du parcours complet.

### 2.2 Cible positive différée — pas autorisation d'effets

Routes confirmées dans `services/IAMRusty/http/src/lib.rs::create_router`, avec préfixe mesh `/iam` :

- **Inscription de A et B** : `POST /api/auth/signup`, JSON `{email,password}`. Nouveau compte : **202**, `user.id`, email attendu, `registration_token`, `requires_username` ; ne pas accepter la branche utilisateur existant200 comme succès d'une fixture neuve. Puis `POST /api/auth/complete-registration`, `{registration_token,username}` :200 et identité attendue. Le jeton de registration reste privé ; l'émission éventuelle de tokens n'est pas un login password prouvé.
- **Login** : `POST /api/auth/login`, `{email,password}` ; après complétion et prérequis email établis :200, identité exacte A/B et tokens présents. Vérifier aussi le refus avec mauvais mot de passe ; compte/email non vérifié reste un état distinct, pas un faux test de mauvais credential.
- **Refresh** : `POST /api/token/refresh`, `{refresh_token}` ;200, champs `access_token`, `expires_in`, nouveau `refresh_token`, `refresh_expires_in`. Réutiliser le nouveau jeton pour `/api/me` ; rejouer l'ancien refresh et vérifier son refus selon le contrat/IT courant. Une réponse identique ou un5xx ne devient pas une preuve de rotation sûre.
- **Principal** : `GET /api/me` avec le Bearer de A puis B :200, ID/email du compte correspondant. Avec token A et en-têtes de principal prétendant B, jamais identité B ; sans Bearer, des headers forgés ne donnent pas une session. Les noms de headers réellement concernés et les codes attendus sont figés d'après la configuration/middleware courants, pas inventés. Refus explicite ou identité A ne doivent pas être confondus ; choisir l'attendu exact du trajet.
- **Négatifs bornés** : email mal formé, password faible/champ requis absent selon DTO ; mauvais credential ; Bearer absent/malformé ou signature invalide pour `/me` ; refresh malformé et refresh consommé. Pour chaque cas : code exact et absence d'identité/token inattendu, à fixer d'après handler/extracteur/IT, pas « tout non200 » ni un ensemble opportuniste400/401/403/422/500.

**Écart de continuité refresh vérifié :** `TokenResponse` possède le membre Rust `token`, mais `handlers/token.rs:25` porte `#[serde(rename = "access_token")]` ; le champ JSON courant est donc **`access_token`**, contrairement à l'indication parent « token en réponse ». Le constructeur du handler ne suffit pas à déterminer la sérialisation. Aucun changement produit ni acceptation opportuniste des deux noms ; cet écart reste signalé pour le futur parcours positif.

Une VU/itération bornée, cas nommés et résultat du **même run** ; checks et complétude explicites. Pas de quota hérité des26 oracles OAuth. Deux comptes, quand ils seront admis, prouveront une isolation de **principaux humains sur `/me`**, pas l'isolation d'organisations/tenants ni les signers internes.

**Expiration distincte :** un token malformé/signature fausse n'est pas un JWT expiré valide. Tester l'expiration exige une clé de fixture correspondant à une clé effectivement publiée ou une TTL de test explicitement autorisée, sans rotation ni modification de l'horloge hôte. Sans ce prérequis, expiration et refresh avec access expiré sont **hors preuve** ; ne pas fabriquer un vert ni importer une clé produit pour satisfaire le test.

## 3. Vérification email et deux fixtures : préparation externe, sans bypass caché

**Blocage actuel confirmé par le parent :** `registration_service::complete_registration` émet `UserSignedUp` via outbox vers Telegraph. Supprimer le user par cascade et l'outbox n'annule pas notifications/queues déjà émises. **Aucune rétention résiduelle n'a été accordée** ; on diffère donc **avant création/complétion des comptes**, sans nettoyage d'autres services. Les propositions ci-dessous restent une cible future sous condition d'inverse causal ou d'accord humain explicite de rétention, pas une préparation admise dans la tranche active. **Aucun SQL n'est nécessaire ou livré pour cette tranche.**

Avant toute écriture, choisir la voie en lisant les vrais DTO/usecases, stockage des tokens, fixtures et FK/outbox. Les sources relues confirment qu'un login non vérifié est refusé ; elles ne qualifient pas une voie opérateur de livraison email dans le lab actuel.

**Préférence :** inscription/complétion réellement stimulées par k6 ; obtention native et privée du token de vérification **généré par le produit** pour les deux seuls IDs/emails, puis appel `GET /api/auth/verify` avec les paramètres `email`/`token` du DTO courant. Ne pas supposer qu'un hash DB est un token utilisable ou qu'un compte existe avant son inscription.

Si cette préparation doit intervenir après signup, utiliser deux phases **fermées** sous le même `k6-run iam` : inscription → étape native de vérification → sessions/login/refresh/me. Pas de SQL privilégié depuis JS, callback générique ou nouvelle API ops. Les tokens restent dans la mémoire de leur phase ; seul le contexte privé nécessaire, notamment les IDs, passe entre phases.

**Proposition SQL autorisée à concevoir**, pas SQL déjà ratifié/exécuté : uniquement les deux fixtures identifiables du run, par script externe paramétré, connexion privée/TLS et contrôles de cardinalité/IDs. Une mise en état email vérifié, si retenue faute de token exploitable, intervient **après** les assertions d'inscription et est déclarée fixture : elle ne prouve pas le comportement de vérification email. Aucun préseed des comptes destiné à remplacer le signup testé, password hash inventé, nom de table/champ supposé, désactivation globale des contrôles ou modification de clé/TTL partagée.

La réduction prévue est **appliquée au périmètre cible** : §2.1 seulement ; inscription/login/refresh positifs déclarés différés. Ne pas maquiller cette réduction en campagne IAM complète. Reset/resend restent hors tranche ; seul le contrôle de disponibilité username en lecture est retenu.

## 4. Secrets, effets et cleanup

Configuration et credentials dans **fichiers privés protégés**, hors Git/build ; chemins explicites, pas latest. Dans le futur parcours positif seulement, A/B seront uniques au run avec IDs exacts mémorisés après création ; une collision bloque. Passwords/tokens restent privés, jamais dans argv, variables d'environnement portant leurs valeurs, console, tags URL, résumé, métriques ou résultats publics. Les réponses d'erreur ne sont pas exportées brutes ; pas de debug HTTP/traces contenant headers ou query de vérification. Un chemin/CA publique peut être transmis au processus, pas la valeur d'un secret.

**Tranche active :** cleanup limité aux fichiers temporaires et processus/forwards réellement possédés ; **aucun cleanup DB ni SQL compensatoire**, puisque aucun compte n'est admis à la création. Pour la cible différée, un inverse devra couvrir les effets causaux, pas seulement FK/outbox : une cascade seule n'est pas suffisante. Pas de DELETE par préfixe email, TRUNCATE, reset de DB/namespace, adoption de données existantes ou intervention dans Telegraph/queues pour annoncer un cleanup complet. Le plein parcours attend cet inverse ou une rétention explicitement discutée et accordée.

`clean` ferme seulement fichiers temporaires, processus/forwards possédés vérifiés et effets attribuables, sur succès, échec et interruption ; il ne coupe pas le lab keepalive. Secrets hors résultat ; petit inventaire privé des effets encore présents si blocage. **Échec cleanup = verdict distinct et sortie globale en échec**, même si les assertions métier passent. Une rétention résiduelle exige un accord explicite, pas un PASS de nettoyage.

## 5. Livraison bornée et hors scope

Le parent peut écrire le petit JS du §2.1 et le PowerShell prepare/run/clean, avec smoke TLS réel avant les cas IAM et **sans SQL**. Implémentation **partielle visée**, pas constat de livraison ou verdict runtime ; ancien harness conservé. Critères : lectures et négatifs précis réellement discriminés, résultats frais/complets/expurgés, états setup/tests/cleanup séparés, aucune garde locale/TLS/secrets perdue. Le parcours complet A/B et ses fixtures ne deviennent implémentables avec effets qu'après levée du blocage causal. Pas de nouveaux tests unitaires de source/harness ou tests Rust imposés par ce contrat.

Hors tranche : OAuth/provision fournisseur, Chromium/diagnostics, SAN mutateur, rotation/signers et routes internes org/S2S, autres services, couverture exhaustive sécurité/multi-tenant, performances, CI, suppression générale du Python historique. Le nouveau parcours IAM n'appelle pas l'ancien framework ; les autres consommateurs ne sont pas retirés dans ce lot.

**GO d'écrire ≠ GO de run/réparation.** Aucun serveur, test, migration ou cleanup exécuté par l'architecte ; aucun verdict runtime. Une nouvelle ADR n'est pas nécessaire pour ce raccord Windows interne borné ; aucun Statut Accepted, protocole métier, schéma produit ou contrat Linux modifié.

## Sources ciblées et limites

- `services/IAMRusty/http/src/lib.rs::create_router` ; `handlers/auth.rs::{SignupRequest,SignupResponse,LoginRequest,CompleteRegistrationRequest,signup,login,verify_email,complete_registration}` ; `handlers/token.rs::{RefreshTokenRequest,TokenResponse,refresh_token}` ; `handlers/user.rs::get_user` ; `platform_user.rs::PlatformUser`.
- `services/IAMRusty/tests/auth_email_password.rs`, cas login non vérifié et DTO d'inscription ; `tests/public_refresh_expired_access.rs`, distinction JWT réellement expiré/refresh public. Lecture ciblée, tests non relancés ; mappings d'erreurs et seed/inverse SQL à préciser par le parent depuis handlers/IT/fixtures, sans nouvelle exploration transverse imposée.
- Continuité parent : émission `UserSignedUp` vers Telegraph, absence d'inverse causal et de rétention accordée ; comportement Windows Go `systemVerify`. `TokenResponse`/attribut serde recoupés par Serena et lecture locale ciblée de `handlers/token.rs:21–32` ; aucun appel API ou run k6 pour cette rédaction.
- [Launcher natif et limites de qualification](kind-local-native-launcher.md) ; dernier état API rapporté dans le [ledger opérateur](../../.cursor/handoffs/20261110-kind-local-operator-cycle.md). Le futur ledger `.cursor/handoffs/20261111-k6-iam-native.md` appartient au parent, pas une entrée d'autorité ni un fichier modifié ici.
