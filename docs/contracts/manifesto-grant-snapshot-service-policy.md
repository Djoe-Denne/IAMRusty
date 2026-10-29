# Politique explicite du vérificateur service grant-snapshot Manifesto

- Version 1, 2026-10-29. Auteur : architecte/pool-4 ; **Accepted / Partial** : code intégré, unitaires et IT ciblées natives vérifiés ; livraison Linux et démarrage de l'image corrigée encore non qualifiés.
- Ratification explicite par le lead, sous délégation utilisateur, après lecture intégrale du document architecte de 80 lignes et examen des sources, alternatives, compatibilité et oracles. Aucune preuve de démarrage réussi n'en est déduite.
- Canon : [ADR0314](../adr/0314-principal-authority-through-openfga.md) et [frontière AuthZ](authz-trust-domain-boundary.md). Précision locale d'un consommateur service distinct, sans supersession ni nouvelle ADR.
- Source : application fad3f8d97f5ebfbf4470aff5663974b451266b82, SDK02e5db23aac6016aab062c3078696d5f7fd464ef. L'auteur a vérifié les sources, pas le runtime. Original architecte conservé dans son worktree sous ce même chemin.

## Décision et alternatives

Conserver le vérificateur SDK et AuthUser exclusivement sur la route service dédiée.
Ajouter à GrantSnapshotAuthConfig un seul champ public
`service_principal_issuer: Option<String>`, absent par défaut, y compris Default Rust.
L'absence refuse le démarrage ; aucun repli vers issuer HS, IAM ou historique.

Pour CET extractor seulement, le root passe ce champ dans JwtAuthConfig.platform_issuer
et fixe allow_historical_platform_issuer=Some(false). Le nom SDK désigne ici l'autorité
unique de la vue UUID de l'acteur technique local ; ce n'est pas une identité
HumanAccount IAM. Ni policy utilisateur, FGA, memberships, événements, cache, ni autre
route UUID-only ne peut recevoir ce mapping. L'autorité n'est jamais déduite du sub.

La route, l'issuer et le protocole existent ; manque l'attestation explicite du root.
Alternatives rejetées : copier la policy IAM (autorité différente), inférer la policy
depuis issuer HS/token/Host ou activer legacy (contredit0314), constructeur pure/test
sans policy, désactiver la route. Un nouvel acteur typé et middleware serait plus
large et introduirait un nouveau chemin de preuve sans nécessité pour ce correctif.

## Configuration et validation avant démarrage

```toml
[apparatus.grant_snapshot_auth]
issuer = "aiforall-platform"
audience = "manifesto-bindings"
service_principal_issuer = "aiforall-platform"
```

Le secret dédié reste dans sa source actuelle ; aucun nouveau secret ou GET Secret.
Le champ est Some, non vide, non paddé, 1..=256 octets UTF-8 et exactement égal à
l'issuer attendu. Appliquer PrincipalPolicy existante. Issuer et audience non vides,
non paddés, sans normalisation. Une configuration opérateur différente doit déclarer
explicitement les deux mêmes domaines exacts et aligner son client : aucun alias ni
auto-alignement. Secret absent/vide ou policy absente/invalide/mismatch : erreur opaque
au startup avant HTTP/lookup. La duplication issuer/policy est volontaire : paramètre
crypto d'une part, attestation d'autorité pour la vue UUID d'autre part.

Fixer allowed_algorithms=[HS256] explicitement. Issuer/audience/clé dédiés inchangés,
pas JWKS, mesh ou clé RSA comme autre autorité, pas de flag permettant legacy=true.
Lazaret conserve clé, audience, TTL, format JWT et sujet UUID de service. Aucune
migration DB, token, tuple ou grant, aucune nouvelle API SDK ou mint.

## Isolation et refus

L'extractor reste sur le seul GET /api/projects/{project_id}/bindings/{component_id},
avec les préfixes standalone/monolithe existants. Les autres routes Manifesto gardent
leur policy IAM et leur garde HumanAccount. Aucun changement de transport/Envoy/FGA.

JWT service valide : vraie crypto, puis acteur UUID local. Même sub avec issuer IAM,
org, autre service ou iamrusty ; mauvais aud/alg/clé/expiration, bearer absent/invalide :
refus avant commande, lookup, RPC ou effet. Headers mesh/UUID injectés ne remplacent
jamais le bearer, pas de with_default_user_id ou constructeur pure/test.

query.principal reste l'utilisateur d'intersection explicite, pas le sub service.
Sans query, conserver le snapshot existant. AuthUser peut rester dans CommandContext
car GetBindingGrantSnapshotCommandHandler ne l'utilise pas comme sujet d'intersection
ou FGA. Si un usage account/membership/permission de cet acteur est trouvé en aval,
STOP et remonter le chemin : le présent contrat ne l'autorise pas.

## Allowlist et migration

- Manifesto configuration/src/lib.rs : champ Option defaultNone et validation locale/tests.
- Manifesto setup/src/app.rs::grant_snapshot_user_id_extractor : validation et policy dédiée, HS256-only ; tests locaux.
- Manifesto config/{default,development,test}.toml : déclaration explicite cohérente, pas valeur inventée dans la bibliothèque.
- Tests apparatus_p3_t4_gateway.rs, apparatus_p3_t5_consent.rs et fixture commune, uniquement cette frontière ; fichier de test dédié dans le même service si attribué par le lead.
- Lecture sans édition par défaut : HTTP route/handler bindings, commande snapshot, client/config Lazaret, monolithe employant le même Application.
- Le full hérite des TOML Manifesto. Si une surcouche remplace ce bloc, attribuer avant édition le seul champ non secret nécessaire ; pas nouveau secret ni patch global renderer/mesh.

Ancienne config personnalisée sans nouvelle déclaration : refus fermé et ajout
opérateur requis. Aucun rollback réactivant UUID sans policy. Standalone et monolithe
partagent le builder, sans raccourci permissif.

## Oracles à inscrire avant exécution

- grant_snapshot_requires_explicit_service_principal_policy : absent, blank, paddé, mismatch, >256, incomplet refusés opaquement ; positif dédié avec policy IAM globale différente.
- grant_snapshot_service_jwt_preserves_uuid_actor_and_query_principal : vrai JWT/routeur, sub A différent de query B, résultat attestant B ; positif sans query.
- grant_snapshot_rejects_foreign_authority_before_snapshot_lookup : même clé/sub mais issuers étrangers, mauvais aud/alg/clé/exp/absent, UUID/headers seuls ; compteur lookup inchangé.
- grant_snapshot_policy_does_not_leak_to_human_account_routes : JWT service refusé sur route utilisateur, positif IAM légitime ; localiser le refus, pas une erreur terminale étrangère.
- Config réellement consommée default/test et builder monolithe commun ; Lazaret wire-compatible par refs/diff. Réutiliser P3T4/P3T5 utiles, pas tout Apparatus ni nouvelle infrastructure.

Validation proportionnée : units configuration/setup puis IT P3T4/P3T5 si affectées,
check Manifesto/monolithe selon diff. Windows natif, 24 jobs, E:, un Cargo ; IT fixtures
existantes, pas Kind. Aucun test ou succès CrashLoop déduit de ce contrat.

## Sources

GrantSnapshotAuthConfig ; setup grant_snapshot_user_id_extractor et appel Application ;
SDK UserIdExtractor::{new,from_parts,extract_user_id}, middleware with_principal ;
HTTP binding_snapshot_routes/handler ; GetBindingGrantSnapshotCommandHandler ;
Lazaret authorization_token/sign_grant_snapshot_jwt/fetch et tests P3T4/P3T5.
Le lead référence ce contrat avant publication du correctif. Aucun SDK, persistence,
protocole ou modèle FGA changé.
