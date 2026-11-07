# OAuthConnect — injection HTTP, compression et secrets résolus

Date : 2026-11-07. Clarification du [contrat actif, §4](oauthconnect-multi-instances.md#4-transport-secrets-et-composition), sous ADR0413 **Accepted / Partial**, pour le lot transport OAuth/secrets autorisé. Base T0 intégrée : `b8b1b8aee2f5c1438537b15e0a6bf276280bc905` ; SDK `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`. Aucun code, test ou runtime exécuté/livré par cette note.

## 1. Décision : injection par composition root de confiance

**Option A retenue.** `OAuthClient::new(&ProviderConfig, SecretValue, reqwest::Client)` reçoit un client de confiance fourni par `setup::build_http_client`. Ce n'est pas une frontière avec du code/plugin non fiable : IAM, un callback, un JSON vendor ou un paramètre réseau ne fournit jamais cet objet Rust. Le contrat exige la politique du **vrai client composé**, pas une introspection impossible des options d'un client arbitraire déjà construit.

En conséquence :

- **Signatures T0 inchangées**, pas de nouveau type capability, constructeur de transport parallèle dans infra, cycle infra→setup, SDK override ou registre de builders.
- Builder de production **unique** dans `setup/src/transport.rs` : redirect `none`, total10s incluant lecture, connexion3s, transport/CA selon configuration explicite. Réemployer les constantes existantes d'infra. Pas de client reconstruit par requête, de fallback `Client::new()`, ni de délai supérieur ajouté par un RequestBuilder.
- La version reqwest utilisée est **0.12.28**, avec retry protocolaire par défaut : configurer explicitement `retry(reqwest::retry::never())` sur le client vendor pour respecter l'obligation existante de non-rejeu du code, sans nouveau retry manager. Pas de changement du SDK pour cela.
- Infra garde ses propres contrôles : déclaration OAuth supportée, URLs/transport, allowlist redirect exacte, paramètres v2/PKCE, méthodes d'auth, corps borné, projection et budget. Le client injecté ne remplace aucun de ces contrôles. Infra ne prétend pas vérifier les options internes reqwest via Debug, réflexivité ou requête réseau de détection.
- Le caller Rust qui injecte volontairement un client suivant les redirects/désactivant TLS ou change ses timeouts viole la précondition d'injection ; on ne promet pas de rattraper ce mauvais câblage dans `OAuthClient`. La preuve **B puis C** doit exclure ce câblage dans le produit conservé. Une fixture A avec son client explicitement configuré ne suffit pas à cette preuve de composition.

**Compatibilité : détail de composition de la cible acceptée**, aucune nouvelle ADR/acceptation humaine requise. Le contrat dit déjà « client injecté par setup » et `setup` reste l'unique composition root (ADR0102). Une garantie future contre un appelant Rust non fiable imposerait une autre frontière/type et serait à cadrer avant changement ; elle n'est pas introduite dans ce lot. L'attente QA A-5 « client arbitraire mal configuré rattrapé » doit être remplacée par l'oracle du builder réellement consommé, pas transformée en test impossible ou contournée par un second builder.

## 2. Q-1 — gzip vendor et lecture décompressée bornée

Le parent retient le positif gzip et le plafond **décompressé**. Activer `gzip` localement sur la dépendance reqwest des packages OAuthConnect consommateurs (`infra` pour son transport/tests et `setup` pour le builder), **sans changer la version ni les features de la déclaration workspace globale**. Le builder vendor fixe explicitement `.gzip(true)` et désactive les autres décodeurs via `.no_brotli()`, `.no_deflate()`, `.no_zstd()` : aucun encodage supplémentaire ne devient supporté parce qu'une autre dépendance active une feature.

Gzip est décodé par **reqwest/tower-http**, pas par un décompresseur maison. Les octets de `Response::chunk()` sont ceux du corps après cette couche. Dans reqwest0.12.28, un gzip reconnu entraîne aussi suppression de `Content-Encoding` et `Content-Length` : leur absence **ne signifie donc pas absence de décompression**. Ne pas refuser indistinctement tous les corps sans ce header ni annoncer PASS par rejet de tout gzip.

Politique au point de lecture exposé par reqwest : absence de `Content-Encoding` ou valeur résiduelle `identity` admise ; toute valeur résiduelle autre qu'identity, invalide, vide ou liste d'encodages est refusée avant parsing. Examiner toutes les valeurs résiduelles du header, pas seulement la première. Un header `gzip` encore visible n'est pas une preuve de décodage : refuser plutôt que tenter un second décodage. Le témoin nominal emploie `Content-Encoding: gzip` reconnu par la version retenue ; pas d'annonce de prise en charge de variantes/empilements que son middleware ne traite pas.

**Limite concrète de cette API** : le middleware peut retirer/normaliser les en-têtes avant que le helper les voie (il sélectionne une entrée Content-Encoding puis la retire). Ce raccord ne constitue pas une validation exhaustive des en-têtes bruts multivaleurs reçus sur le fil. Il garantit les encodages activés sur le vrai client, le refus des encodages résiduels et la borne sur les octets effectivement livrés au parsing. Ne pas fabriquer un accès aux headers pré-décompression ni promettre leur inspection ; une exigence supplémentaire de rejet de toute combinaison brute malformée demanderait un autre raccord à cadrer séparément, pas un nouveau décompresseur dans ce lot.

**Un helper de lecture dans infra**, partagé par token POST et APIs GET :

1. Refuser non2xx sans recopier le corps distant dans l'erreur, et contrôler les headers résiduels. Un Content-Length/hint peut aider à refuser tôt un dépassement connu, jamais autoriser un corps ni dimensionner sans borne un buffer.
2. Lire par `chunk().await` sur la réponse du client configuré ; comparer la taille du chunk à `HTTP_BODY_LIMIT - buffer.len()` **avant extension**, ou addition vérifiée équivalente. À plafond+1, erreur et abandon de réponse ; pas de drainage non borné, résultat partiel ou relance du code.
3. Après EOF seulement, parser le buffer borné puis valider les DTO/projeter. Un gzip invalide/tronqué, une erreur de stream ou de JSON produit l'erreur existante sanitized. Aucun `.bytes()`, `.text()`, `.json()` ou `collect()` non borné préalable, ni lecture intégrale compressée pour décompresser ensuite.

La borne est **1_048_576 octets retenus décompressés par réponse**, inclusive. Elle ne prétend pas que le middleware/allocateur ne possède jamais déjà un chunk, ni un plafond RSS/CPU global. Le budget JSONPath existant reste obligatoire après parsing et ne remplace pas cette borne HTTP.

### Unification Cargo et SDK

Les features Cargo sont additives pour les consommateurs du même reqwest0.12.28 ; une déclaration locale n'isole pas son effet lors d'une compilation combinée. Le parent contrôle le delta lock ciblé et les arêtes de compression ajoutées, sans update global, bump reqwest ou SDK. Les builders legacy GitHub et IAM qui laissent les encodages par défaut sont des consommateurs repérés : ne pas annoncer une absence d'impact workspace sans inspection/check ciblé du graphe conservé, ni une qualification de tous ces anciens chemins.

**OpenBao reste inchangé** : le SDK fixe déjà `.no_gzip().no_brotli().no_deflate().no_zstd()`, redirect none, no-proxy, total5s/connexion3s, réponse64KiB et token/valeur16KiB. Son `get` refuse l'encodage nonidentity et lit borné avant JSON. L'activation locale gzip ne doit pas lui retirer ces réglages. Réemployer ce client SDK, pas le client vendor10s/1MiB pour le coffre. Ajouter une sentinelle coffre gzip refusé sous le graphe activant gzip ; pas de mutation SDK pour faire passer ce témoin.

## 3. Q-3 — normalisation HMAC héritée, une fois en résolution

`SecretValue` SDK conserve exactement le texte UTF-8 nonblank reçu et le refuse au-delà de16KiB ; il ne trimme, ne décode ni ne sérialise le secret. Cette propriété reste inchangée. La règle supplémentaire HMAC est **locale à OAuthConnect**, identique en direct et OpenBao :

1. Acquérir/valider la valeur **brute** via `SecretValue`/résolveur SDK, y compris la borne16KiB **avant** tout trim. Pas d'acceptation d'une valeur brute trop grande sous prétexte que sa version trimmée serait courte.
2. Dans `setup::resolve_secrets`, appliquer **une seule fois** `str::trim()` sur la valeur HMAC : même traitement Unicode des bords que l'ancien `GitHubConnect::Application::new` et que l'adapter HTTP IAM pour ses clefs UTF-8. Aucun changement des caractères intérieurs, de casse ou de normalisation Unicode, aucun décodage base64.
3. Exiger **au moins16 octets UTF-8 après trim**, non16 caractères. Retourner la valeur canonique dans le champ T0 `ResolvedProviderSecrets.hmac_secret: SecretValue`, reconditionnée avec le wrapper SDK existant si nécessaire. Ne pas modifier le SDK ou le texte de la config chargée pour obtenir ce résultat.
4. C transfère **ces mêmes octets** à `HmacKey::new`, sans seconde normalisation. `HmacKey` est un wrapper, pas le validateur de longueur. La composition doit conserver cet invariant, pas fabriquer de clef par défaut.

`client_secret`, lui, reste **exact**, y compris espaces de bord nonblank, dans les deux modes et jusqu'au POST Basic/form correctement encodé. Ne pas lui appliquer le trim HMAC. Le bootstrap token-file relève encore des règles distinctes du SDK (une terminaison LF/CRLF autorisée, pas trim générique).

La validation déclarative actuelle vérifie la longueur HMAC brute et le nonblank ; elle ne prouve pas la force de la clef résolue. Une déclaration brute de16+ octets dont le trim laisse15 octets doit donc échouer au stade B. Ne pas changer les114oracles antérieurs pour masquer cette étape de validation résolue.

**Compatibilité héritée**, pas un nouvel arbitrage crypto : GitHub appliquait déjà trim avant longueur/consommation ; le contrat demandait le réemploi de cet acquis. Modes direct/vault passent par la même normalisation finale. Chaque instance garde son association id/secrets ; aucun secret partagé implicitement, mais **pas de nouveau refus d'égalité entre instances**, ni obligation de dédupliquer les appels coffre/références. Les fixtures distinctes prouvent le bon routage, pas l'interdiction d'un partage explicitement configuré.

## 4. Q-2/Q-4 — preuves réalistes et erreurs

- **A** prouve les vraies opérations OAuth contre collaborateurs sortants WireMock, avec un client de fixture explicitement fiable si B est encore refusant ; aucun mock du service/usecase lui-même. Son plafond de lecture est démontré sur les chemins token **et** GET, avec petit gzip positif, JSON gzip au plafond et plafond+1 décompressés, gzip invalide et encodage résiduel non supporté même si le body serait du JSON valide. Les JSON paddés au plafond gardent une projection simple sous budget ; pas de stress/harness neuf.
- **B** prouve la vraie résolution SDK et le vrai builder : politiques constantes, transport/CA, encodages et retry explicités. Normalisation HMAC par témoins direct/vault identiques : bords retirés, intérieur conservé, trim15 refusé, trim16 accepté, multioctets comptés en octets ; `client_secret` exact. Les valeurs résolues restent non sérialisables et Debug masqué, y compris dans un batch défaillant.
- **C**, après intégration A+B, prouve le chemin **build_http_client → OAuthClient::new → requêtes réelles**, avec la clef HMAC résolue correctement consommée sur PATH complet. Notamment : redirect vers destination pourtant approuvée non suivi (zéro reçu destination), support gzip réel, CA/SAN corrects et erreurs TLS fermées, secret de chaque instance vers son endpoint seulement. Ne pas substituer en C un client reconstruit par un helper test.
- Réemployer le **pattern** TLS local `IAMRusty/tests/support/verified_idp_tls.rs` (PEM/CA/SAN, loopbackport0, tâche possédée et join gracieux), pas son protocole S2S ni ses types reqwest0.11 dans le client0.12. Petit helper local à la suite concernée, pas nouveau harness global ni test IAM complet.
- Une seule preuve contrôlée du délai vendor>10s, et une du coffre>5s, suffisent au lot. Compteur/phase atteinte et erreur timeout réelle, avec contrôle nominal et watchdog de sécurité ; un watchdog seul ou `is_err()` quelconque n'est pas un PASS de timeout. Ne pas multiplier les attentes pour chaque statut/instance. Le point de mesure peut être B ou C, à condition de prouver ensuite son raccord réel, pas de répéter artificiellement le même délai.
- **Mesure connexion3s** et **stream sans Content-Length** restent des GAPS explicites si les moyens locaux existants ne permettent pas des témoins déterministes. Source du builder actif/consommateur C et inspection de la boucle de chunks sont des preuves **structurelles**, pas une mesure réseau/mémoire. Ni adresse Internet « silencieuse », nouveau serveur raw/stress, ni délai artificiellement élargi pour fermer administrativement ces gaps.
- Employer les `DomainError`, `FederatedOAuthError`, `SecretError` et conversions existants ; le socle refusant reste un vrai RED nominal, les négatifs déjà refusés des caractérisations. Ce lot sans serveur vérifie `Err` typé/absence de résultat partiel, **pas** de nouvelles enveloppes HTTP502/404. Erreurs/logs/chaînes source ne recopient pas corps, headers, code, verifier, tokens, nonce, valeur ou référence de secret. Pas de nouveau framework d'erreurs.

## 5. Ownership, séquencement et limites

- **A** : `infra/src/oauth.rs`, helper de corps dans `infra/src/transport.rs`, ses tests `oauth_transport`/TLS locaux. Pas de résolution coffre ni builder de production en infra. Garder le mapping/budget livrés.
- **B** : `setup/src/secrets.rs` et `setup/src/transport.rs`, tests `startup_secrets`/builder et helpers coffre. Pas d'implémentation de l'OAuth vendor dans B. A/B disposent des APIs T0 intégrées et peuvent écrire leurs tests indépendamment ; pas d'import setup dans infra pour les coupler.
- **C** : `setup/src/composition.rs` et `minimal_composition`, seulement après intégration réelle des deux producteurs. `build_oauth_clients` OAuth-only échoue explicitement avant acquisition/publication si une déclaration requiert le client OIDC absent, conformément au socle ; `resolve_secrets` peut traiter les secrets des deux profils. Aucun catalogue amputé ni listener/registry opérationnel promis. Config OpenBao globalement présente pour une instance n'interdit pas les instances directes valides : exclusivité par instance, pas copie de l'ancien test monoinstance appliquée à tout le batch.
- Exports/dépendances partagées et root Cargo/lock coordonnés par le parent avant consommateurs ; **aucune signature T0 à changer** pour cette clarification. Aucun author ne modifie un helper partagé en concurrence. Parent intègre cette annexe et réconcilie Q1–Q4/A5 dans le plan QA avant RED A/B, puis valide C après A+B.

Restent hors lot : OIDC runtime/Discovery/JWKS/UserInfo, nonce transactionnel et raccord IAM, déploiement/Kind, retrait legacy, claims riches/actions et E2E externes. Aucune obligation de bascule nouvelle, aucune promotion de Réalité ADR0413 et aucun Accept implicite de0505. Les preuves doivent conserver RED puis GREEN sur mêmes oracles et distinguer les gaps des résultats exécutés.

## 6. Sources vérifiées et absence d'exécution

### Photographie après implémentation du lot — parent, 2026-11-07

Cette note initiale n'avait livré aucun code ; son protocole a depuis été implémenté et vérifié dans la tranche transport/secrets. `OAuthClient`, résolution SDK, vrai builder et composition OAuth-only sont intégrés. Gate final189tests (31transport+33secrets/builder+11composition+114régressions), fmt/check/Clippy strict verts ; revues sécurité ciblées sans finding. Le seul warning SDK est préexistant, hors des sources modifiées.

Le timeout vendor est maintenant mesuré via le vrai client composé C, avec contrôle nominal, token11s, watchdog15s non déclenché, erreur ExchangeCode9–11s et POSTunique/GET0. Le témoin de fixture A équivalent a été remplacé après preuve C, pas supprimé pour masquer un échec. SDK5s, gzip/coffre refusé, gzip vendor POST/GET, CA/SAN et transfert HMAC PATH/corps exacts sont également exercés. Le helper public `configuration::validate_provider` réemploie les mêmes validations locales ; `validate_config` conserve unicité et vrai bloc global OpenBao. Aucun second loader/validateur ou secret factice en infra.

Les limites des sections précédentes restent effectives : connexion3s non mesurée, streaming sans longueur absent, pas RSS/SLO ni validation exhaustive des headers bruts pré-décodeur. Tests WireMock/TLS loopback, pas provider/OpenBao réel. La composition ne publie ni listener ni registry et refuse OIDC avant acquisition ; IAM/OIDC/E2E restent hors lot.

Source T0 lue au canon **et** fichiers documentaires du worktree, même HEAD b8b1b8a ; MCP Serena canonique conservé, sans activate_project/indexation. APIs : `services/OAuthConnect/infra/src/{oauth,transport}.rs`, `setup/src/{secrets,transport,composition}.rs`, validation déclarative `configuration/src/validation.rs:71–96`. Historique HMAC : `services/GitHubConnect/setup/src/app.rs:Application::new`, `services/IAMRusty/infra/src/auth/http_connector.rs:with_security_mode_and_roots`, `crates/idp-connect-contract/src/hmac.rs:HmacKey::new`.

SDK canonique (sous-module non initialisé dans ce worktree documentaire, non modifié) : `rustycog/rustycog-config/src/secrets/mod.rs:SecretValue::new`, `OpenBaoKv2Resolver::{connect,get}` ; no-decompression, limites et acquisition KV v2 réellement relues. Analogue composition root : ADR0102 ; acquis secrets : ADR0505 Proposed/Implemented, non ratifiée par ce réemploi.

Bibliothèques aux versions du lock, sources registry lues : reqwest0.12.28 `async_impl/client.rs` (builder/décompression/retry), `async_impl/response.rs:Response::chunk`, `retry.rs:never` ; tower-http0.6.11 `decompression/future.rs:35–76`. Références primaires versionnées : [ClientBuilder](https://docs.rs/reqwest/0.12.28/reqwest/struct.ClientBuilder.html), [Response::chunk](https://docs.rs/reqwest/0.12.28/reqwest/struct.Response.html#method.chunk), [retry::never](https://docs.rs/reqwest/0.12.28/reqwest/retry/fn.never.html), [middleware décompression](https://docs.rs/crate/tower-http/0.6.11/source/src/decompression/future.rs).

Preuves du lot à établir par les acteurs autorisés, pas par cette note : aucun Cargo/check/test, endpoint fournisseur/coffre, listener TLS/WireMock ou runtime démarré ici. Aucun code applicatif, test, SDK, QA, ledger, wiki ou digest modifié ; aucune revendication de nouveau RED/GREEN/PASS.
