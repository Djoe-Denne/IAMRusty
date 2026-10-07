# Inventaire de simplification signing/admission — autorité entreprise conservée

2026-10-06. **Proposition non contraignante**, aucune ADR créée/modifiée/Accepted, aucun retrait produit ratifié.
Périmètre exhaustif visé : responsabilités signing/admission/publication IAM et leurs frontières ; pas tous les fichiers du dépôt. Les symboles listés sont des points de travail, pas une liste de suppressions autorisées maintenant.
Lecture statique seulement : aucun code, tests, Cargo, runtime, Git, installation ou sous-agent. Sources précédentes réutilisées ; aucune recherche standards/Marvin/Transit répétée.

## 1. Besoin et vocabulaire

Clé plateforme commune par défaut ; entreprise opt-in avec clé dédiée/KMS/HSM. L'utilisateur confirme désormais qu'une organisation derrière un HSM doit pouvoir faire accepter ses propres tokens : **autorité indépendante**, pas seulement custodie. C'est l'organisation/système qui demande l'enrôlement ; le HSM effectue des opérations crypto, il n'est pas un principal humain autonome.
Une signature étrangère valide n'entraîne jamais l'auto-trust. IAM/administration autorisée lie publiquement provider/keyref/version/kid à l'issuer/owner/scope. L'organisation ne peut émettre que dans son domaine accepté, pas comme principal plateforme ou d'une autre org.
Les flux normaux plateforme n'ont pas à changer d'identité/token à chaque org UI. L'émission/switch org reste incomplète dans la source actuelle ; ne pas vendre cette proposition comme déjà livrée.

## 2. Décision proposée sur les octets

**Supprimer la comptabilité exacte répétée du jeu complet : oui après remplacement. Supprimer toute borne de publication : non.**
Recommandation : **slots de coût maximal fixe**, avec cap global d'epochs publiables, réserve plateforme et caps par trust domain ; JWKS matérialisé/sérialisé par révision ou transition/expiration, pas recalculé depuis tous les PEM pour chaque admission ou mint.
Une epoch Pending, Active ou Retiring non expirée occupe un slot. Revoked et Retiring expirée libèrent selon horloge DB et mutation atomique ; révoquer ne réinitialise pas le churn. Une org non opt-in ne consomme pas un slot de signing.
Un slot couvre la pire représentation JSON de l'entrée sur tous ses états atteignables : RSA/public n/e, métadonnées, échappements UTF8/JSON, champs optionnels et changements de status. Validation/canonicalisation une fois par nouveau matériau/binding ; aucun champ non borné ne doit contourner la preuve.
La preuve doit établir **enveloppe + séparateurs + slots plateforme + slots org <= plafond writer**, et dimensionner les slots org en conservant la capacité réservée plateforme. Cap par org seul est insuffisant avec un nombre d'orgs non borné. Pas de nouveau nombre de slots choisi ici.
Le serializer réel reste la référence : `.len()` du payload produit est un garde-fou final peu complexe, pas le calcul coûteux répété pour tous les variants/toutes les clés. Un dépassement doit refuser la mutation AVANT retrait de l'ancien Active/activation, pas publier incomplet ni découvrir trop tard une panne globale.
Conserver provisoirement les bornes ratifiées : writer786432octets, réserve plateforme65536, budget global org720896, entrée réservée4096, caps16plateforme/8org et churn4nouveauxkids/3600s. Ce ne sont pas des quotas de nombre d'entreprises ;720 du test est une borne de boucle.
**Trade-off** : slots maximaux = moins dense, plus de refus conservateurs. Si les champs actuellement permis ne garantissent pas un slot sûr, garder une validation de taille ponctuelle de l'entrée ou proposer explicitement des bornes plus strictes ; pas de preuve imaginaire issue des seuls nombres4096/1024.
Alternative moins conservatrice : conserver les coûts variables mais les calculer/mémoriser une fois par binding et maintenir des totaux transactionnels. Elle préserve mieux la capacité, mais conserve davantage de comptabilité. Ce n'est pas la recommandation maximale de simplification.

## A. Extraire vers OpenBao/KMS/HSM

- Génération de clés et nouvelles versions, stockage privé, protection/non-exportabilité, signature RSA privée. IAM peut demander une opération sans générer ni recevoir une clé privée.
- Import/export privé ou provisioning fichier lorsqu'un provider le permet : responsabilité d'exploitation du provider, pas upload de PEM privé à IAM production. BYOK externe peut n'autoriser que Sign/GetPublicKey : IAM n'est pas toujours autorisé à Rotate.
- ACL, identités/credentials de service, journal d'audit crypto et politiques du provider : gestion dans l'outil standard ; IAM garde sélection des références, moindre privilège et logs métier sans secrets. Toute la gestion du trust/credentials ne disparaît pas magiquement.
- Le provider peut créer/renouveler du matériel ; IAM orchestre les versions admises, preuve public/privé, publication et activation. Un rotate externe seul ne met pas à jour notre JWKS et ne révoque pas les JWT hors ligne.
- Réutiliser `SigningProvider`, `TransitSigningProvider`, `RemoteSigningProvider`, `WorkloadIdentity`, pas un nouveau framework ou broker. Le port minimal Sign/GetPublicKey existe déjà.

## B. Retirer d'IAM production APRÈS remplacement

- `mint_pem_material` et génération `RsaPrivateKey::new`, arborescence fichiers/écriture PKCS8 privés dans `infra/src/signing/rotate.rs` ; remplacer par opération provider autorisée.
- Chemin RSA privé de `PemSigningProvider::sign_digest` et son chargement privé dans le setup production. Conserver PEM dev uniquement comme mode explicite isolé si choisi ; jamais fallback automatique après panne KMS. Ne pas confondre retrait des opérations privées et suppression de toute dépendance RSA utilisée pour public parsing/tests.
- Provisioning/import de secrets privés maison devenu inutile. Ne pas supprimer les références de credentials, HTTPS/egress ou audit métier.
- Travail dupliqué PEM→RSA public→JWK/DTO et copies de representations dans `domain/src/entity/token.rs` ; garder une forme publique canonique validée et réutilisable par matériau/version.
- `SigningKeyLifecyclePolicy::reserved_entry_bytes` comme tarification exacte par candidat/jeu complet, et les rescans/reparses de `publication_usage`/`check_admission` à remplacer selon le contrat de slots. Source actuelle : trois variants Pending/Active/Retiring, pas quatre.
- Générations RSA répétées des fixtures qui ne testent pas génération/renouvellement : matériau neutre préparé, sans supprimer signatures/probes/clefs distinctes des vrais scénarios. Les IT provider réels restent ; les stubs ne deviennent pas une preuve OpenBao réel.
- La préparation cumulative de la boucle `signing_admission` peut devenir seed d'un état valide proche de la nouvelle frontière + mutations réelles de refus/race/reserve. Les assertions et scénarios sont remappés explicitement au nouveau contrat, pas supprimés pour faire baisser le temps.
- Arrêter les extensions cloud/provider non livrées inutiles au périmètre immédiat ; ne pas enlever silencieusement enum/DTO/API publics, `RemoteSigningProvider` ou WorkloadIdentity déjà consommés. Aucune campagne globale de dépendances/fixtures n'est proposée.

## C. Simplifier/remplacer, pas supprimer

- Budgets dynamiques par clé → slots/global count et réserve avec preuve JSON maximale ; writer atomicité inchangée. Comptage indexé/COUNT SQL ou compteurs transactionnels, pas chargement de toute table à chaque admission ni compteur local process.
- Sérialisation complète/parsing répétés → métadonnées/entrées publiques préparées, snapshot calculé lors mutations ET expirations, invalidé par epoch. Lecture de snapshot validé pour GET ; mêmes statuts/predicate/horloge que writer.
- Churn : requête indexée/compte agrégé de fenêtre, sans recopier/trier toute l'histoire. Garder l'histoire pertinente malgré revoke. Un rate limiter admin/backpressure ne remplace le quota ratifié qu'après preuve d'équivalence et accord humain ; pas de limite mémoire par replica.
- Idempotence : binding complète sous lock, pas création d'une nouvelle clé à chaque retry ; no-op sans vieillissement, kid ni nouveau coût. Cela évite des rotations accidentelles sans retirer les rotations demandées.
- Rotation : provider keyref+VERSION épinglée, kid public opaque par epoch. Adapter Transit actuel ne transmet pas `key_version`; utiliser les versions est une évolution à contracter, pas déjà garanti.
- Provider I/O/probes : pas de lock DB long autour du réseau. Revalider epoch/binding/capacité au commit après I/O ; opération retardée ne peut ni réactiver une révocation ni signer sans fence. Préparer/réserver ne vaut pas Active.
- Les états pourraient être représentés plus simplement en interne, mais Pending/prepublication/probe/Retiring ne disparaissent pas tant que leur rôle n'est pas remplacé et prouvé. Aucun retrait automatique de writer registry, mutation epoch, publisher/outbox ou fence.

## D. Garder impérativement

- PlatformIdentity unique multi-org, principal `(iss,sub)`, Hive/OpenFGA pour membership/permissions, distinction context UI vs switch explicite de trust domain.
- OrganizationManagedIdentity/modèle de trust indépendant maintenant demandé, issuer org et owner/kid/alg binding ; ne pas créer une identité par org métier ordinaire. L'issuer reste contrôlé par le contrat, pas choisi dans les claims d'un token reçu.
- Garde compte plateforme : principal org valide ne doit pas pouvoir me/link/relink un compte plateforme. Trust n'accorde pas automatiquement des rôles ; le provider ne vérifie pas les autorisations métier.
- Registry de **métadonnées publiques**/binding (référence privée opaque), preuve possession du privé correspondant au public avant trust/activation, public matériel canonique, unicité kid et entrées bornées, alg/iss/aud/typ explicitement vérifiés.
- Prépublication N+1, activation atomique, rétention N jusqu'à vie restante+skew, révocation/stop emission, fraîcheur maximale60s, fail-closed et fencing après signature distante. Pas de fallback plateforme sur panne du domaine enterprise.
- Cap global de publication, réserve plateforme, quotas finis d'epochs/churn ou remplacements équivalents ratifiés ; isolation inter-org/inter-replicas. Limites du provider ne garantissent pas la taille du JWKS partagé.
- Credentials scoped/least privilege, TLS et egress/SSRF policy, pas de secrets dans events/erreurs. Entrées/import public d'un HSM ne sont pas auto-trust.
- Tests trust, JWKS, lifecycle, races/refus sans dommages, compte plateforme, provider crypto/transport. Les fixtures peuvent être moins coûteuses ; les garanties ne disparaissent pas.
- Seed/cache JWKS de bootstrap : sujet fonctionnel distinct. Retrait d'org signing ne corrige pas les401 froids, et retirer seulement PEM org ne remédie pas au chemin PEM plateforme. Aucun verdict sécurité/PASS nouveau ici.
- Protocole workload Apparatus/Lazaret dédié et ses contrôles live Manifesto : hors inventaire de suppression IAM org, ne pas les recoupler à un JWT humain/clé entreprise.

## E. Ne pas introduire sans consommateur/contrat

- Nouveau service signing, broker ou moteur générique de policies/claims.
- STS/RFC8693/context-token et cache(session,org,aud) simplement pour changer d'org UI.
- Login/Identity/JWT neuf pour chaque membership plateforme ; adaptation SSO qui confond fournisseur de login et autorité de signature.
- Matrice universelle KMS/vendor ou auto-trust de token/HSM étranger ; ni quotas provider présentés comme preuve de capacité globale.
- Externalisation totale AuthN/OIDC à Transit : Transit ne remplace pas claims/comptes/sessions/refresh/publication métier.

## 3. Sources, ratification et validation future

- `services/IAMRusty/domain/src/entity/signing_key.rs:354–459` : `reserved_entry_bytes`, `publication_usage`, simulation/reparse complète ; `:473–491` histoire/tris churn.
- `services/IAMRusty/infra/src/signing/rotate.rs:197–260` : keygen PEM local ; Transit crée actuellement un nouveau nom/kid, pas une rotation de version déjà épinglée.
- `services/IAMRusty/infra/src/signing/transit.rs:187–198` : digest SHA256/PKCS1v15 sans `key_version`.
- `services/IAMRusty/domain/src/port/signing.rs:18–40` et `docs/adr/0309-remote-signer.md:12–27` : couture provider minimale existante.
- `docs/adr/0304-jwt-acces-plateforme-rs256-jwks.md:25–34` ; `docs/adr/0305-account-identity-trust-domain.md:24–30` ; `docs/adr/0306-hive-iam-configuration-signature.md:22–36` : autorité org, compte/multi-org, administration.
- `docs/adr/0310-admission-epochs-budget-jwks.md:23–27,45` : chiffres/atomicité et accord humain requis pour substitution ; `0308-mesh-authn-jwt.md` : fraîcheur/révocation.
- Reuse : `docs/local/20261006-signing-scope-review.md` et `docs/local/20261006-org-authorization-vs-trust-review.md` ; écart strict-reject org du SDK demeure documenté, pas réparé ici.
Impact : architectural/cross-module/API-contrat (IAM domaine/infra/setup/migration, publisher, Hive signer clients/routes, monolith bridge, SDK/ext-authz et tests selon contrat). Pas une simple suppression locale.
**Ratification requise avant code** : modifier/superséder précisément0310 pour le calcul et la nouvelle capacité ; amender0304 si PEM production retiré, et0306/0309 selon opérations/version/API changées. Aucune Accept ni nouveau fichier ADR maintenant.
**Preuve minimale future** : borne JSON tous variants+échappements+envelope ; dernière place race inter-org, réserve plateforme, fill jusqu'à la frontière, refus sans retrait, promotion/no-op, churn malgré revoke, expiry/libération, probe retardé/revoke, N/N+1 verifiable et writer fence ; validation legacy/migration avant activation. Suites ciblées IAM signing_admission/jwks/org account guard/provider, SDK JWT/trust, ext-authz et Hive/monolith selon impact. Rien exécuté ici ; aucune estimation de gain.
Seul compromis produit laissé ouvert par cette recommandation : accepter la capacité plus conservatrice des slots plutôt que la densité de l'exact accounting. Les nombres de slots se fixent après preuve et accord humain, pas par cette note.