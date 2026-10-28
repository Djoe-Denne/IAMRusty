# Contrat ratifié — transport sécurisé du signataire distant

- Statut : **Accepted / Unimplemented** ; ratification explicite du lead le 2026-10-28 sous délégation utilisateur de la mission auth v1, après examen des deux propositions architecte. Aucun test fonctionnel déduit de la ratification.
- Date : 2026-10-28. Auteur : architecte, lot `arch-signataire`.
- Jalon : clôture sécurité AuthN / IAM signing, hors P-Apparatus.
- Base examinée : application `c72ae362b950c63d6736a6818c68babc5a8d6f40` ; SDK/gitlink `d60a88b2f2f499768a807941a1b4c80c9a366b3f`.
- Canon : [0309](../adr/0309-remote-signer.md) **Accepted / Partial**, [0311](../adr/0311-crypto-privee-deleguee-aux-providers.md) **Accepted / Implemented** ; [0304](../adr/0304-jwt-acces-plateforme-rs256-jwks.md), [0306](../adr/0306-hive-iam-configuration-signature.md), [0307](../adr/0307-workload-identity-port.md).
- Réalité de ce contrat : exigences non encore implémentées ni testées par ce lot documentaire. Aucun changement de Statut/Réalité des ADR.

## 1. Décision, alternatives et seuil ADR

**Durcir le fournisseur HTTP existant, sans le désactiver par défaut.** Son activation sécurisée reste supportée, sous les conditions ci-dessous. Une configuration non conforme est refusée explicitement ; elle n'est jamais ignorée pour continuer sur PEM, Transit ou une autre clé.

Alternatives examinées :

- **Retenue :** HTTPS authentifié, destination stable, zéro redirection et réponses bornées. Réutilise Sign/GetPublicKey, WorkloadIdentity et le port existant ; ferme les écarts sans enlever une fonctionnalité acceptée.
- Interdire tout RemoteHttp en `Verified` jusqu'à un vendor HSM qualifié : non retenu. 0309 accepte déjà le contrat minimal et son adapter ; l'absence de vendor n'impose pas la suppression du fournisseur HTTP supporté. Ce durcissement ne qualifie toutefois aucun matériel ou service de production.
- Garder HTTP ou redirections en `Verified` au motif du digest-only / bearer : rejeté. Le digest n'authentifie pas le serveur et le credential reste sensible ; TLS et destination de l'appel sont des contraintes indépendantes.

**Aucune nouvelle ADR (0315 non consommée).** Ce contrat précise les contrôles d'un canal et d'un provider déjà bornés par 0309 et les garanties TLS/egress/erreurs de 0311 §Décision. Pas de nouveau fournisseur, protocole public, stockage ou port domaine ; aucune supersession. Le lead ajoute le lien de ce contrat à la référence/index 0309 lors de son intégration, sans réécriture silencieuse des Accepted. Toute extension ultérieure du trust, du protocole ou de la persistance exige un nouvel arbitrage.

## 2. Configuration et activation

Le mode est `SecurityMode` existant : `Verified` par défaut, `LocalInsecure` ou `IsolatedTest` uniquement sur sélection explicite. Ni debug/release, ni hostname, ni variable d'environnement de profil ne changent ce mode implicitement.

- Activation inchangée : `jwt.backend = "remote"` ou son alias `jwt.provider` sélectionne le fournisseur ; un bloc `jwt.remote` seul ne l'active pas. Préserver cette compatibilité.
- Si backend et provider non vides se contredisent et que l'un sélectionne remote, refuser l'ambiguïté avant résolution de secret/I/O plutôt que masquer remote derrière une priorité historique.
- En sélection remote, URL et `key_id` non vides, configuration WorkloadIdentity exploitable, binding/public et preuve de possession existants sont obligatoires. Une sélection contradictoire remote/Transit ne doit pas choisir silencieusement un autre provider : la refuser avant I/O.
- Algorithme de signature remote admis : **RS256 uniquement**, digest SHA-256 du signing input, pas d'autodétection ou de repli HS256/PS256/autre. En mode `Verified`, le vérificateur effectif reste RS256-only selon 0304 ; ne pas ajouter une fenêtre HS256. Les tests legacy explicitement isolés hors remote ne sont pas supprimés.
- Le digest Sign fait exactement **32 octets** ; refus avant WorkloadIdentity/I/O. Le credential obtenu doit être non vide et produire un en-tête valide, sans panic sur caractères invalides ; Authorization sensible, aucune réflexion du credential dans l'erreur.
- `jwt.workload` conserve les mécanismes déjà supportés par `compose_workload_identity`. WIF explicite incomplet/inconnu ou résolution échouée : refus, jamais fallback static après échec. L'absence de WIF peut conserver le `StaticCredential` explicite existant (`jwt.remote.token`, non vide), conformément à 0307. Ce contrat ne livre ni mTLS client, ni SPIFFE, ni nouveau cloud exchange.
- Valider la configuration **avant résolution des credentials, lecture PEM, requête réseau ou mutation de registry**. Une URL interdite doit produire zéro appel WorkloadIdentity et zéro requête au signer.

### URL admise

Utiliser le parseur `url::Url` déjà dépendance de `iam-configuration`, pas `starts_with("https://")` comme validation complète.

- URL absolue HTTP(S) avec host et port valides ; HTTPS obligatoire en `Verified`, y compris vers le loopback. Refuser URL relative, schéma différent, host absent, userinfo (y compris userinfo vide), mot de passe, query ou fragment (même vides), caractères de contrôle, espaces de bord et antislash ambigu. Ne jamais refléter l'entrée dans l'erreur.
- **Seule exception HTTP :** modes explicitement `LocalInsecure` ou `IsolatedTest`, host littéral normalisé `127.0.0.1` ou `[::1]`. Aucun nom DNS (même `localhost`), wildcard, IP privée LAN, alias de conteneur/Service, ni résolution DNS vers loopback ne crée une exemption. Pas de flag permettant HTTP en `Verified`.
- HTTPS accepte un host DNS ou une IP configurés par l'opérateur et vérifiés par TLS ; les destinations privées ne sont pas globalement interdites, les signers internes étant légitimes. Ce n'est pas une preuve d'allowlist réseau, de défense DNS-rebinding ou d'absence générale de SSRF.
- Une base avec préfixe de chemin est conservée : `/sign` et `/keys/{key_id}` sont ajoutés sous ce préfixe, sans changer schéma/host/port. Le `key_id` reste opaque dans le JSON ; dans le GET il constitue **un seul segment encodé**, jamais un fragment d'URL à joindre. Refuser `key_id` vide ou les segments `.` / `..` ; aucun séparateur réservé ne doit permettre d'échapper au préfixe.
- La destination provient uniquement de la configuration de confiance IAM, jamais de claims, `kid` de JWT reçu, réponse du signer ou paramètres utilisateurs. Pas de découverte/follow-up d'URL renvoyée par le serveur.

### TLS et client

- TLS **1.2 minimum** ; validation normale de la chaîne, du nom DNS ou de l'IP SAN de l'URL, et des dates. Aucun `danger_accept_invalid_certs`, `danger_accept_invalid_hostnames` ou équivalent, dans aucun mode HTTPS.
- Réutiliser les racines de confiance du backend TLS déjà résolu ; aucune installation globale de CA, aucun nouveau magasin ou champ CA production dans ce lot. Une CA de fixture peut être injectée seulement dans un client de test borné, sans désactiver chaîne/nom.
- Client dédié remote : `redirect(Policy::none())` **pour Sign et GetPublicKey, dans tous les modes**. Tout 3xx est un refus, même même-origin, URL HTTPS ou downgrade local ; aucun second appel, aucun credential/digest transmis à la cible de redirection. Désactiver aussi le Referer automatique.
- Ne pas hériter de proxy système ou `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY` pour ce client (`no_proxy()`). L'exception loopback ne doit pas traverser un proxy ambiant. Un déploiement imposant un proxy devra expliciter son besoin hors de ce contrat, pas contourner TLS ou ajouter un fallback.

## 3. Protocole, délais et lectures bornées

Le protocole reste `POST {base}/sign` avec JSON `key_id`, `algorithm`, `digest` base64 standard, et `GET {base}/keys/{segment}` avec réponse PEM publique brute ou JSON `public_key`. `Authorization` vient uniquement de WorkloadIdentity. **Ni claims/JWT complets, ni clé privée envoyés au signer.**

### Délais

- Le timeout HTTP **10 secondes existe déjà** : le conserver, depuis la connexion jusqu'à la fin du corps, sans le réinitialiser à chaque chunk.
- Ajouter une borne globale **10 secondes par opération** Sign/GetPublicKey depuis l'entrée de l'opération, couvrant résolution WorkloadIdentity, I/O et lecture bornée. Le timeout HTTP est imbriqué dans cette même échéance, non additionné ; une résolution de credential suspendue ne bloque pas indéfiniment. L'analyse JSON/PEM/base64 porte uniquement sur les petits buffers bornés ci-dessous.
- Pas de retry automatique de l'adapter, de délai cumulatif caché, de repli de transport/provider/clé. L'échéance provoque une erreur et l'abandon de la future ; elle ne garantit pas qu'une opération déjà reçue côté signer n'a jamais eu lieu.
- Ces bornes sont des budgets de l'adapter, pas un SLA de login, un engagement de latence CPU sous saturation ou un mécanisme d'annulation distante.

### Corps : plafonds fixes, inclusifs

- Succès Sign (2xx) : **4 096 octets** maximum du corps complet avant parsing JSON/base64.
- Succès GetPublicKey (2xx) : **16 384 octets** maximum du corps complet, PEM brut ou JSON, avant parsing/décodage.
- Erreur HTTP, y compris redirection : **0 octet lu/conservé par l'application**. Retourner uniquement opération/catégorie/statut numérique ; abandonner la réponse sans `text()`, `bytes()`, `json()`, drainage ou copie d'un extrait. Aucun diagnostic fournisseur n'est requis par le protocole actuel.

Ces valeurs sont des décisions de ce contrat, pas des constantes prétendument déjà livrées. Une signature RSA8192 vaut 1 024 octets (base64 1 368) ; un public RSA8192 et son enveloppe JSON/PEM disposent d'une marge confortable. Le plafond public n'est **pas** la taille d'un slot JWKS (0312), et ne remplace ni les validations RSA2048–8192 ni les quotas/publications existants.

Implémentation attendue :

1. Refuser immédiatement un `Content-Length` connu supérieur au plafond, mais **ne jamais compter sur sa présence**.
2. Lire les chunks en comptant les octets réellement livrés ; vérifier le dépassement avant chaque ajout au buffer avec calcul sans overflow. À exactement N : accepté si le protocole est valide ; à N+1 : refus, sans parser ni retourner un préfixe tronqué.
3. Même comportement sans Content-Length, transfert chunked ou fin par fermeture ; EOF prématuré/erreur de transport = refus. Aucun `response.json/text/bytes` non borné sur un succès.
4. Désactiver la décompression automatique du client avec les options `no_gzip/no_brotli/no_deflate` disponibles dans reqwest 0.11.27, demander `Accept-Encoding: identity`, refuser tout encodage de contenu autre qu'identité. Ainsi une future unification de features ne réintroduit pas de décompression implicite avant le compteur.
5. Buffer applicatif retenu au plus N ; une bibliothèque transport peut matérialiser un chunk avant ce contrôle. Ne pas annoncer un plafond RAM global du processus ni une borne des buffers TCP/TLS.
6. Parser JSON depuis les octets bornés ; PEM brut en UTF-8 strict, sans substitution silencieuse de caractères. Conserver le refus de toute clé privée et de tout matériau non public. La validité RSA/public, la preuve de possession, le binding et la vérification de signature par IAM restent autoritatifs.

## 4. Confidentialité et erreurs

- Messages de validation constants, `SecretError::InvalidFormat` ; échecs d'appel via l'erreur externe `remote_signer` existante, catégorie/opération et éventuellement statut numérique. Aucune nouvelle erreur publique détaillant URL, keyref, token ou réponse brute.
- Ne pas propager le `Display` brut de reqwest, du parseur d'URL, de base64/JSON ou d'une erreur WorkloadIdentity susceptible de contenir une valeur hostile/sensible. Étiqueter sans recopier l'entrée ; conserver une causalité non secrète si utile.
- Interdits dans logs/traces/rapports : Authorization/secret/WIF brut, corps de requête/réponse, digest/signature, JWT/claims, URL complète, `key_id`/référence sensible, PEM privé. Marquer le header credential sensible. Ne pas ajouter de Debug/Serialize d'un client/config exposant le token.
- L'interdiction concerne aussi les corps HTTP d'erreur et les Location hostiles. Les tests utilisent des sentinelles synthétiques et assertent leur absence dans les erreurs/captures de logs pertinentes ; jamais de vrai secret dans un artefact.

## 5. Invariants, impact et compatibilité

Impact **cross-module / contrat de configuration de sécurité** : `iam-configuration` → `iam-infra` → `iam-setup`, identique standalone et monolithe via le composition root IAM. Aucun nouvel appel métier ni format Sign/GetPublicKey, route, event, migration SQL, modèle OpenFGA, SDK ou fournisseur.

- `SigningProvider` / `WorkloadIdentity` domaine inchangés ; digest-only et credentials opaques préservés.
- Registry/version/public/admission, preuve de possession et contrôle autoritatif **après** I/O distante restent en place. Une signature reçue ne permet pas de contourner une révocation ou un changement de binding intervenu pendant l'appel.
- Le setup recopie remote.key_id dans le kid public : préserver la contrainte d'admission UUIDv4 simple32hex existante au bootstrap. Les identifiants arbitraires des tests d'adapter ne prouvent pas une admission complète ; aucune dissociation key_id/kid ni nouvelle représentation persistée dans ce lot.
- Aucun fallback PEM, autre provider/clé, bootstrap après registry initialisé vide ou remplacement d'un binding incompatible. Remote reste non versionné au protocole actuel, contrôlé contre son public canonique ; ne pas inventer `latest` ou une version fournisseur 1.
- Aucun changement aux 60 secondes de fraîcheur/révocation, JWKS vide autoritatif, rotation/récupération atomiques, quotas, permissions ou frontières Hive→IAM.
- Migration de configuration seulement : HTTP existant en `Verified` doit passer à HTTPS valide avant reprise ; une fixture HTTP loopback sélectionne explicitement `IsolatedTest`. Pas de conversion automatique vers un mode moins sûr, de correction automatique d'URL ou de migration de données.

### Allowlist du consommateur `corriger-signataire`

- `services/IAMRusty/configuration/src/lib.rs` : `validate_signing_security`, `remote_http_endpoint`, validation commune et units. Un helper remote dédié dans ce même module est permis ; `security.rs` seulement si nécessaire pour exposer/réutiliser une règle, **sans changer la politique des connecteurs**.
- `services/IAMRusty/infra/src/signing/remote.rs` : constructeur, destination, client, échéance, lecture bornée, erreurs.
- `services/IAMRusty/setup/src/app.rs` : uniquement passage du mode validé au constructeur remote, avant I/O ; pas de refonte setup/Transit/registry.
- `services/IAMRusty/infra/tests/remote_signer.rs` et `services/IAMRusty/tests/fixtures/remote_signer/{mod,resources,service}.rs` : cas de régression et petits auxiliaires de transport/TLS dédiés. Pas de nouvelle infrastructure globale.

API d'adapter : conserver la signature existante de `RemoteSigningProvider::new(...)`, mais avec défaut **Verified**. Ajouter une entrée explicitement nommée et typée `new_with_security_mode(..., mode: SecurityMode)` pour setup et fixtures ; aucune inférence depuis l'URL. Utiliser une validation commune configuration/constructeur pour éviter une voie directe moins sûre. La sécurité ne doit pas dépendre du seul appel préalable du setup. Pas de constructeur public acceptant arbitrairement un client reqwest non validé.

Hors allowlist : domaine, migrations, SDK/gitlink, manifests/Envoy, Transit/WIF providers, tests des autres membres, handbook/index/ledger/politique. Tout besoin découvert est présenté au lead pour ownership avant édition, pas absorbé silencieusement.

## 6. Validation exigée du consommateur (non exécutée ici)

Ajouter ces noms au plan QA avant leur exécution ; des tests paramétrés peuvent regrouper les variantes sans diminuer les oracles.

1. `remote_endpoint_security_modes_are_explicit` : HTTPS valide admis en Verified ; HTTP loopback refusé en Verified, accepté seulement dans les deux modes explicites ; HTTP DNS/LAN refusé partout. Tester alias backend/provider, URL seule inactive, URL invalide/schéma/userinfo/query/fragment/whitespace et binding sélectionné contradictoire. Refus avant resolver/réseau/lecture PEM ; témoin valide conservé.
2. `remote_constructor_is_secure_by_default` : appel direct à `new` ne contourne pas Verified ; constructeur explicite local/test opérationnel. `remote_rejects_unsupported_algorithm_before_io` : aucune requête avec algorithme non RS256.
3. `remote_redirects_never_reach_target` : pour **les deux opérations**, 301/302/303/307/308, même-origin et autre-origin/downgrade ; serveur initial réellement appelé, cible avec **zéro requête**, pas seulement Authorization absent. Témoin direct 200 réussit.
4. `remote_success_bodies_are_bounded_without_content_length` : Sign et public JSON/PEM, N valide/N+1 refusé, avec longueur annoncée et vrai flux sans Content-Length (chunked et fermeture). Une fixture qui recrée Content-Length ne satisfait pas cet oracle. Le dépassement après un préfixe JSON valide doit être refusé, pas tronqué. `remote_rejects_encoded_responses` couvre encodage non-identité.
5. `remote_http_errors_are_not_read_or_reflected` : 3xx/4xx/5xx avec corps très grand, segmenté/lent et sans Content-Length ; erreur immédiate après headers, pas de drainage/écho, sentinelles absentes. Distinguer absence de lecture applicative et éventuel buffering de transport.
6. `remote_operation_deadline_covers_identity_and_body` : WorkloadIdentity suspendue, headers lents et corps lent/fragmenté, borne globale sans redémarrage par chunk ; aucune requête si credential jamais obtenu. Tolérance d'ordonnancement Windows documentée, pas un seuil implicite de SLA.
7. `remote_tls_authenticates_server_name` : vrai échange TLS du client utilisé par le provider, même CA de fixture : bon nom accepté, mauvais nom refusé ; CA inconnue refusée. Racine injectée au seul client de test, jamais vérification désactivée. Ce test ne prouve **pas** le comportement d'Envoy (lot distinct).
8. Préserver les six tests actuels `remote_sign_sends_digest_not_claims`, `remote_get_public_key`, `remote_uses_workload_identity_header`, `remote_url_absent_is_fail_closed`, `remote_never_returns_private_key`, `remote_rejects_private_key_from_remote`. Ajouter `remote_errors_redact_hostile_inputs` et témoin public RSA8192 ; une signature stub de protocole n'est pas une preuve cryptographique complète.

Commandes ciblées futures : `cargo test --locked -p iam-configuration --lib`, `cargo test --locked -p iam-infra --test remote_signer`, puis fmt/Clippy/check IAM selon le diff et le plan du lead. Si le wiring change, vérifier la compilation IAM/monolithe et réutiliser/compléter seulement les régressions signing réellement affectées. Windows natif, Cargo unique, 24 jobs, cible E:. Dans ce lot documentaire, la première tentative de commit a déclenché involontairement le hook `.githooks/pre-commit` exécutant `cargo fmt --all -- --check` : **FAIL environnement Windows OS206** (« nom de fichier ou extension trop long »), avant création du commit. Aucun test comportemental, autre commande Cargo, Docker/Kind/WSL ni publication exécutés ; aucun PASS fmt revendiqué. Le lead a autorisé, pour cette seule livraison documentaire, un commit avec `core.hooksPath=NUL` sur cette invocation uniquement, sans modification persistante du hook/config/permissions. La QA Rust finale reste requise selon le plan de mission.

Le consommateur rapporte ROUGE/VERT sur le même oracle, SHA/diff testé, cas non exécutés et échecs d'environnement séparés. Aucun PASS fournisseur/HSM/production ou clôture auth globale ne découle du contrat ou d'une compilation.

## 7. Sources vérifiées et limites

- `configuration/src/lib.rs` : `JwtConfig::validate_signing_security` (323–373), `remote_http_endpoint` (520–539) ; remote vérifie aujourd'hui activation/non-vide, contrairement au contrôle HTTPS Transit. `configuration/src/security.rs` : modes explicites ; `validate_connector_url` (186–206), analogue de validation URL relevant du flux IdP (plage 0409), à réutiliser comme motif, pas à élargir silencieusement.
- `infra/src/signing/remote.rs` : `new` (42–85), `authorization` (87–95), `sign_digest` (100–135), `public_key` (137–179). Timeout 10 s déjà présent ; redirections par défaut ; succès JSON/texte non bornés. **Les corps HTTP d'erreur ne sont actuellement pas lus** : préserver ce comportement, ne pas prétendre avoir corrigé une lecture d'erreur inexistante.
- `setup/src/app.rs` : `remote_signing_bits` (1504–1594), `require_platform_remote_binding` (1406–1418), `signing_auth_after_policy` ; références Serena vers setup/reexport/tests recoupées. `domain/src/port/signing.rs` : contrats digest/public et WorkloadIdentity ; `domain/src/entity/signing_key.rs` : borne RSA8192. Préfixe commun : `services/IAMRusty/`.
- `infra/tests/remote_signer.rs` : six tests présents lus/inspectés ; fixtures `tests/fixtures/remote_signer/service.rs` : WireMock HTTP sortant. Présence ≠ résultat d'exécution. Aucune couverture TLS/chunked/redirection déduite de ces tests.
- `docs/platform/authn-jwt.md` : annonce remote Partial ; son fallback bootstrap/JWKS vide reste une divergence documentaire déjà hors de ce lot, pas une autorisation de le réintroduire. Wiki `obsidian/AI FOR ALL/projects/aiforall/decisions/0304-access-jwt-trust.md` : conception/photographie seulement ; 0306 y est encore annoncée Implemented, contrairement à son canon Partial.
- Versions : `services/IAMRusty/infra/Cargo.toml` dépend de **reqwest 0.11** avec `json` et features par défaut ; `Cargo.lock` résout **0.11.27**, `url` **2.5.8**, `native-tls` **0.2.18**, et comporte aussi les dépendances rustls. La présence de rustls dans le lock ne prouve pas son usage effectif par ce client ; ne pas citer reqwest 0.12 pour ce lot ni changer de backend/dépendance par principe.
- Documentation officielle consultée le 2026-10-28 : [ClientBuilder 0.11.27](https://docs.rs/reqwest/0.11.27/reqwest/struct.ClientBuilder.html) (timeout jusqu'à fin du corps, no_proxy, https_only, min_tls_version, décompression), [Policy](https://docs.rs/reqwest/0.11.27/reqwest/redirect/struct.Policy.html) (défaut dix redirects ; none), [Response](https://docs.rs/reqwest/0.11.27/reqwest/struct.Response.html) (chunk ; content_length optionnel ; text/bytes lisent tout), [native-tls 0.2.18](https://docs.rs/native-tls/0.2.18/native_tls/struct.TlsConnectorBuilder.html) (validation hostname active par défaut). Ces API permettent le durcissement sans nouveau fournisseur.

Exploration symbolique sur la racine canonique Serena, à la base ci-dessus ; nouveaux fichiers du worktree non indexés. Les index ne prouvent pas l'absence de tout consommateur externe. Sources/versions et documentation vérifiées, pas de nouvelle exécution fonctionnelle. Après intégration vérifiée et ajout du lien inverse par le lead, `corriger-signataire` peut consommer ce fichier.

## 8. Ratification et arbitrage des propositions — 2026-10-28

Le lead retient le contrat initial livré par contrat-signataire et les précisions convergentes du second architecte sur digest32, credential valide/sensible, sélection ambiguë et kid existant. La proposition alternative ADR0315/contrat de pool-4 reste préservée dans sa branche, non intégrée et non ratifiée : pas un second canon ni une seconde implémentation.

Arbitrages : **Sign4096B/public16384B**, plutôt que16KiB pour les deux. Une signature RSA8192 encodée base64 fait1368B ;4KiB conserve une marge suffisante sans accumuler inutilement davantage. Les tests à borne exacte restent exigés. **Pas de nouvelle ADR0315** : précision de0309/0311 sans nouveau port, protocole, fournisseur ou stockage. Mode explicite sécurisé par défaut, échéance10s incluant WorkloadIdentity et refus sans fallback sont communs aux deux propositions. Cible ratifiée, implémentation/tests encore absents.

Régressions complémentaires nommées avant exécution : `remote_rejects_invalid_digest_before_credentials_or_io`, `remote_authorization_rejects_invalid_credentials_without_leak`, `remote_verified_requires_rs256_only`. Elles complètent le §6 sans nouvelle plateforme de preuve. Les fichiers IAM configuration/setup sont attribués au seul consommateur remote pendant ce lot ; le lead ajoutera le wiring PrincipalPolicy après son intégration, sans concurrence avec identités.

### Attribution test ciblée après plan du consommateur

Le lead autorise `services/IAMRusty/infra/Cargo.toml` uniquement pour les dev-dependencies rcgen et rustls du workspace, et tokio-rustls **0.26.4 déjà résolu** dans le lock. Ce sont les briques du motif TLS de test existant, pas un nouveau backend produit ni une nouvelle plateforme. Module privé `cfg(test)` dans remote.rs pour ajouter la seule racine de fixture au builder sécurisé réel ; aucun constructeur public acceptant un client arbitraire. Cargo.lock : seule mise à jour des références de dépendances du package iam-infra si nécessaire, sans changement de versions/checksums/registry ni update général. Toute autre dérive revient au lead avant livraison.
