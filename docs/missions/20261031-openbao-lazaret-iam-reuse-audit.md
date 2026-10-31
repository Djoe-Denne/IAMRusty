# Réutilisation OpenBao — Lazaret, IAM et RustyCog

## Conclusion

**Réutiliser le KV existant si la compatibilité est confirmée ; ne pas élargir
le SDK au seul motif que les deux services utilisent OpenBao.**

- **Lazaret** peut déléguer la lecture KV v2 au SDK, derrière son port métier
  existant. Le principal écart est le cycle de vie : lecture pendant une
  opération autorisée, pas snapshot de secrets au démarrage.
- **IAM** utilise réellement **Transit** : signature distante, clé publique
  versionnée, création et rotation. Le résolveur KV actuel ne couvre aucune
  de ces opérations. **Garder Transit dans IAM à ce stade** : un autre
  consommateur indépendant de cette capacité n’est pas démontré par cet audit.
  Plusieurs organisations, clés et versions d’IAM ne suffisent pas à établir
  un besoin de plateforme.
- **Transport et credentials durables sont des problèmes à traiter**, pas une
  justification automatique d’un nouveau module SDK. Commencer par des
  adaptations locales ; extraire seulement une capacité réellement répétée,
  dont les sémantiques communes et le bénéfice d’abstraction sont établis.
  Ni port fourre-tout `SecretResolver`, ni clés privées Transit déplacées en KV.

### Critère directeur précisé par l’utilisateur

Une extraction exige à la fois un usage récurrent suffisamment démontré, des
besoins réellement proches et une couche d’abstraction significative, avec un
gain supérieur au coût de contrat/maintenance. Une API qui ne ferait que
déplacer des appels HTTP OpenBao n’est pas une justification suffisante.

Un port sémantique avec des adaptateurs OpenBao ou autres peut être une bonne
forme **si des usages concrets le justifient**. Ce n’est ni une cible déjà
demandée, ni un prétexte pour implémenter d’autres backends hypothétiques.
L’analyse ci-dessous recense donc des capacités potentielles, sans en déduire
qu’elles doivent entrer dans le SDK.

Le KV fournit deux usages concrets à comparer (GitHubConnect et Lazaret), mais
ce seul nombre ne ratifie pas la migration. Le port SDK actuel masque valeurs,
erreurs et protocole ; sa référence `mount/path/field` reste orientée KV. Une
éventuelle généralisation multi-backend devrait démontrer un vrai contrat
sémantique commun, pas simplement renommer cette forme en référence universelle.

Analyse statique demandée par l’utilisateur, **pas** décision d’architecture,
ADR Accepted, autorisation de migration ou preuve d’exécution des services.
Base : AIForAll `8785cc902507ff8079a746022c7575f00701553d`, SDK/gitlink
`93b890ff7b821e9cb2e7d0aeb81957d098778a2f`, 2026-10-31.

## 1. Ce qui existe dans le SDK

`rustycog::config::secrets`, feature `secrets`, fournit une référence structurée,
`SecretValue` opaque et un port async de résolution. L’adaptateur OpenBao lit
un champ string KV v2. Il n’impose pas lui-même que `resolve` soit appelé au
boot : **le snapshot est le choix de GitHubConnect**, pas une impossibilité
d’appeler le port plus tard.

En revanche, `connect` lit le token-file **une fois**, le conserve et vérifie
`lookup-self` : aucune policy root, TTL restante1..3600s. Ni relecture du fichier,
renouvellement ni nouveau login. Un agent qui remplace ensuite le fichier ne
met pas à jour un résolveur déjà construit. Après expiration, les lectures
échouent : ce n’est pas une solution durable de credentials pour Lazaret/IAM.

Autres invariants réutilisables : HTTPS vérifié, CA additionnelle, exception HTTP
loopback explicite, absence de proxy/redirect implicite, erreurs/Debug expurgés,
timeout5s et lecture incrémentale64KiB. Les valeurs sont non blanches et limitées
à16KiB. Le loader strict additif reste déclaratif et peut aussi être réutilisé
pour éviter de masquer une erreur de configuration des futurs modes explicites.

Sources : [SDK types/port](../../rustycog/rustycog-config/src/secrets/mod.rs#L106),
[connexion/auth](../../rustycog/rustycog-config/src/secrets/mod.rs#L257),
[résolution](../../rustycog/rustycog-config/src/secrets/mod.rs#L394),
[contrat initial](../contracts/startup-secrets-openbao.md).

## 2. Lazaret : réutilisation KV possible, migration à encadrer

### Usage actuel vérifié

- `VaultHttpSecretResolver` lit déjà `GET /v1/{mount}/data/{path}` et extrait
  `data.data[field]` string en `Vec<u8>`.
- Son port appartient au domaine Lazaret : `resolve(&str) -> Vec<u8>`, référence
  `secret:{path}#{field}`, mount configuré séparément.
- Dans `InvokeService`, session, admission et grant sont contrôlés **avant**
  dispatch ; `connector.fetch` résout ensuite la référence et injecte les octets
  au connecteur. Ne pas précharger les secrets au boot ni court-circuiter ces
  contrôles en migrant le transport.
- Absence de configuration Vault : `DeniedSecretResolver`. Cela désactive la
  capacité plutôt que de fournir une valeur de secours ; préserver une sémantique
  explicite désactivée/configurée, pas un nouveau fallback.

### Adaptateur recommandé

Conserver le port métier et le parser Lazaret. Un petit adaptateur infra traduit
le résultat du parser + mount en `SecretReference`, appelle le port SDK et
transfère `SecretValue::into_inner().into_bytes()`. Les erreurs SDK sont mappées
vers les erreurs Lazaret expurgées. Aucune dépendance du domaine vers le SDK.

Ce n’est pas un simple remplacement de type/config :

- **Identité** : token inline actuel contre token-file + lookup-self dans le SDK.
  Le token court doit rester utilisable sur la durée du processus : prévoir un
  fournisseur relisant le sink d’agent ou un login/renouvellement explicite.
- **Grammaire** : le parser Lazaret interdit les points, mais admet `/` dans
  le champ et peut admettre un slash initial/final ; le SDK impose un champ
  littéral sans slash et des segments non vides. Les composants SDK sont aussi
  bornés à1024octets. Ne pas élargir le contrat Lazaret parce que le SDK autorise
  des points ; inventorier/corriger les références incompatibles.
- **Valeurs/tailles** : l’ancien adaptateur accepte une string vide et une
  réponse jusqu’à256KiB (`MAX_PAYLOAD_BYTES`). Le SDK refuse les valeurs blanches,
  limite le secret à16KiB et la réponse à64KiB. Décider les compatibilités, sans
  affaiblir silencieusement les défauts sûrs de GitHubConnect.
- **Transport** : le client Lazaret actuel a un timeout5s, mais pas de politique
  explicite no-redirect/no-proxy/HTTPS ; la borne finale après `.bytes()` n’est
  pas une borne incrémentale en l’absence de Content-Length. Réutiliser le SDK
  apporte ici un gain concret, avec un raccord de configuration nécessaire.
- **Confidentialité config** : `VaultConfig` dérive Debug avec un champ token
  inline. La nouvelle configuration déclarative et expurgée évite cette surface.

Sources : [port/parser](../../services/Lazaret/domain/src/secrets.rs#L17),
[client KV](../../services/Lazaret/infra/src/vault.rs#L26),
[composition](../../services/Lazaret/setup/src/app.rs#L373),
[contrôles avant dispatch](../../services/Lazaret/application/src/invoke.rs#L309),
[injection connector.fetch](../../services/Lazaret/application/src/invoke.rs#L435),
[config](../../services/Lazaret/configuration/src/lib.rs#L164),
[borne256KiB](../../crates/apparatus-contracts/src/limits.rs#L27).

## 3. IAM : mutualiser le protocole Transit, pas sa politique métier

### Usage réellement actif

`TransitSigningProvider` résout un credential via `WorkloadIdentity` avant les
appels distants. Il fournit :

- signature d’un digest SHA-25632octets, `prehashed=true`, `sha2-256`,
  `pkcs1v15`, `key_version` explicite ; décodage exact de `vault:vN:<base64>` ;
- lecture des métadonnées et de la clé publique de la **version demandée**, sans
  fallback sur latest ; vérification RSA-2048/non-exportable/support signature ;
- création initiale avec préflight d’absence, puis vérification des propriétés ;
- rotation du même nom, vérification avant/après du successeur exact, **sans
  retry après une issue ambiguë**.

Le transport est déjà partagé entre les instances scoped et interdit les
redirections, timeout10s. Mais son code n’impose pas HTTPS hors loopback, ne
désactive pas les proxies ambiants et désérialise les réponses sans borne
incrémentale explicite. Une base de transport commune est utile sans régresser
le partage du pool déjà testé.

### Ce qui pourrait être extrait si un besoin récurrent est démontré

Un **client Transit typé, optionnel et distinct du KV** pourrait construire les endpoints
(mount configurable), envoyer le digest/signature versionnée, lire les métadonnées
et clés publiques ; éventuellement primitives create/rotate dans une capacité
d’administration séparée. Conserver validation des enveloppes et versions,
tailles/délais bornés et aucune relance aveugle des écritures.

**Ce n’est pas la recommandation d’implémentation actuelle.** Le client existant
IAM reste en place tant qu’une récurrence indépendante et un bénéfice
d’abstraction ne sont pas établis. Un port générique de signature distante
aurait potentiellement plus de sens qu’un simple wrapper Transit si plusieurs
consommateurs/backends partagent réellement son contrat ; cela reste à démontrer,
sans aplatir artificiellement des différences de versionnement ou de sécurité.
Les primitives create/rotate n’ont pas, à elles seules, de justification SDK
supplémentaire et peuvent rester entièrement locales.

Ne pas appliquer mécaniquement la borne64KiB KV à la lecture Transit : une réponse
`keys` contient l’historique des versions publiques et peut croître. Les bornes
doivent rester finies et explicites par opération ; leur valeur est à établir,
pas à augmenter sans preuve ni à imposer uniformément à tous les clients.

### Ce qui doit rester dans IAM

`SigningProvider`, registre et preuves d’enrollment, sélection de la version
active, scopes plateforme/organisation, liaison key_ref/credential_ref/version,
interdiction des clés Apparatus/Cosign et droits de rotation. Le SDK ne choisit
ni un `kid`, ni une organisation, ni une clé latest, ni une identité de secours.
Les principes « version exacte », non-exportabilité et absence de retry ambigu
restent préservés, même si certains contrôles techniques sont partagés.

**Attention au faux candidat KV** : `SecretStorage::Vault::resolve()` est encore
non implémenté, comme GcpSecretManager. En revanche certains champs de cette
ancienne variante servent de compatibilité URL/token pour la configuration
Transit. Implémenter soudain cette variante comme lecture KV changerait ce
contrat et ne migrerait pas la signature réelle. Le SDK KV peut servir à d’autres
secrets d’amorçage IAM dans un lot distinct, pas à exporter la clé privée Transit.

Sources : [transport/bindings](../../services/IAMRusty/infra/src/signing/transit.rs#L68),
[création/rotation](../../services/IAMRusty/infra/src/signing/transit.rs#L156),
[version publique exacte](../../services/IAMRusty/infra/src/signing/transit.rs#L314),
[signature](../../services/IAMRusty/infra/src/signing/transit.rs#L426),
[binding scoped](../../services/IAMRusty/infra/src/signing/scoped.rs#L75),
[composition plateforme](../../services/IAMRusty/setup/src/app.rs#L1447),
[compatibilité URL/token](../../services/IAMRusty/setup/src/app.rs#L1634),
[SecretStorage legacy](../../services/IAMRusty/configuration/src/lib.rs#L100).

## 4. Identité : ne pas confondre WIF cloud et token OpenBao

IAM a déjà un port `WorkloadIdentity` et des adaptateurs static/AWS/GCP/Azure.
Les providers cloud sont sélectionnés explicitement, sans fallback statique
sur configuration incomplète. Leur principe d’injection est réutilisable ; ne
pas déplacer les erreurs/dépendances IAM dans le SDK ni copier leur implémentation
sans revoir bornes, confidentialité, contrat de credential et audience.

**Écart observé** : GCP/Azure renvoient un `access_token` cloud, AWS le
`SessionToken` STS ; le client Transit utilise directement ce résultat comme
`X-Vault-Token`. Le test `transit_sign_uses_wif_resolved_token` vérifie ce câblage
avec WireMock, pas une authentification réelle OpenBao.

La documentation OpenBao décrit un login JWT recevant `jwt` + `role` et renvoyant
un **token OpenBao dans `auth.client_token`**. Aucun login OpenBao de ce type
n’apparaît dans le chemin étudié. Un token STS/cloud n’est donc pas à considérer
automatiquement comme un token OpenBao ; un éventuel broker externe n’est pas
inspecté ni qualifié ici. C’est un raccord à clarifier avant de promouvoir ces
providers en capacité SDK « authentification OpenBao ».

Besoin minimal pour des services durables : un fournisseur explicite
de **credentials OpenBao** avec provenance/cible et validité, dont token-file
alimenté par un agent en premier. La lecture d’un nouveau fichier/token doit
conserver les contrôles de policy/TTL et échouer sans reprendre un token expiré.
Un login JWT/workload natif peut venir ensuite avec rôle/mount/claims/audience
validés et un test de protocole réel. Pas de moteur universel de cache/rotation
ou de daemon ajouté par défaut ; le mode snapshot de GitHubConnect reste inchangé.
Ce fournisseur peut rester local : son existence dans plusieurs chaînes ne
justifie une extraction que si le contrat de cible, validité et réacquisition
est réellement commun. Le raccord WIF incomplet se traite dans IAM, sans attendre
la création d’une abstraction SDK.

Sources : [port IAM](../../services/IAMRusty/domain/src/port/signing.rs#L77),
[composition](../../services/IAMRusty/infra/src/signing/wif/mod.rs#L79),
[GCP](../../services/IAMRusty/infra/src/signing/wif/gcp.rs#L53),
[AWS](../../services/IAMRusty/infra/src/signing/wif/aws.rs#L56),
[Azure](../../services/IAMRusty/infra/src/signing/wif/azure.rs#L68),
[test WIF Transit](../../services/IAMRusty/infra/tests/wif_exchanges.rs#L91).

## 5. Suite recommandée, sans extraction forcée

1. **Lazaret KV, candidat concret** : étudier un adaptateur du port existant vers
   la capacité SDK déjà livrée. Trancher credential durable et compatibilité
   références/tailles. Conserver admission/grants et lecture au moment autorisé.
   Si le contrat reste trop différent, garder l’adaptateur local est acceptable.
2. **IAM, rester local** : clarifier/qualifier la chaîne d’authentification
   OpenBao et traiter ses besoins de transport dans IAM. Conserver Transit,
   scopes, binding, registre et orchestration ; pas de nouveau client SDK requis
   par le présent constat.
3. **Transport/credentials, candidats conditionnels** : comparer les usages
   réellement nécessaires aux deux services. Une factorisation interne minimale
   peut suffire ; n’exporter une abstraction qu’après preuve de sémantique commune,
   de récurrence et de valeur ajoutée. Pas de framework d’identité universel.
4. **Transit/signature partagée, différé** : rouvrir l’extraction uniquement si
   d’autres consommateurs concrets apparaissent. Aucun nouveau backend pour
   justifier artificiellement le port. Toute décision transverse passerait alors
   par architecte, sans modifier implicitement ADR0505 ou le snapshot GitHubConnect.

Recherche exacte complémentaire sur les sources Rust `services/` : les45
occurrences des chemins Transit sign/keys et du type `TransitSigningProvider`
renvoient à IAM et ses tests. Cela étaye « un seul consommateur démontré ici »,
pas une preuve d’absence dans tout déploiement, langage, backend ou dépôt externe.

### Contrôles à conserver lors d’une future migration

- Lazaret : T6 KV HTTP, T12 OpenBao réel, invoke/grant-deny avant tout accès et
  injection sans stockage métier. Adapter lookup-self/token-file ; la fixture
  T12 démarre OpenBao avec un token root dev, volontairement incompatible avec
  les contrôles SDK : créer une identité de test bornée, ne pas autoriser root
  dans le SDK pour faire passer la fixture.
- IAM : tests `transit_protocol.rs` version exacte, credential manquant sans
  fallback, redirections, timeout, création, rotation ambiguë sans retry ;
  isolation scoped et réutilisation du transport. Vérifier une vraie chaîne
  d’authentification pour tout nouveau provider OpenBao.
- GitHubConnect : aucun changement de son snapshot au boot, du fail-closed ou
  des défauts de transport à la faveur de la généralisation.

Sources tests : [T6](../../services/Lazaret/tests/apparatus_p3_t6_kv.rs#L108),
[T12](../../services/Lazaret/tests/apparatus_p3_t12_openbao.rs#L13),
[fixture dev](../../services/Lazaret/tests/fixtures/openbao/mod.rs#L161),
[Transit](../../services/IAMRusty/tests/transit_protocol.rs#L184),
[rotation ambiguë](../../services/IAMRusty/tests/transit_protocol.rs#L550).

## Sources externes et limites

Documentation primaire consultée le31octobre2026, affichée version2.7.x :
[JWT login](https://openbao.org/docs/api/auth/jwt/#jwt-login) et
[Transit](https://openbao.org/docs/api/secret/transit/#sign-data).
Le choix `key_version=0`/absent signifie latest dans l’API : conserver la version
explicite IAM. Ces pages ne qualifient pas une instance réellement déployée.

Sources et références/consommateurs utiles inspectés avec Serena, matching exact
et Context Mode. Les tests cités ont été **lus, pas exécutés** pendant cet audit.
Aucune compilation ni exécution de tests Rust, aucun Docker, Kind, secret
personnel, coffre réel ou modification de code applicatif/SDK. Le hook de commit
peut vérifier le format Rust sans compiler. Les déclarations de faisabilité sont distinctes d’une
preuve de migration ; aucun gain chiffré ni qualification de sécurité exhaustive.
