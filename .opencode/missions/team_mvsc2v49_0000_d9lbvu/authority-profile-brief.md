# Décision complémentaire — autorité effective du profil OAuth

## Problème nouveau après ratification

Le contrat navigateur intégré est Accepted / Unimplemented ; sa cible HTTP18080 et les hops internes mTLS restent fixés. Le lead a relevé un fait source incompatible avec l'hypothèse « public_base_url18080, issuer plateforme8080 inchangé » : `JwtConfig::platform_issuer()` dérive l'issuer RS256 de public_base_url, pas de jwt.issuer (qui décrit le legacy HS256). Le profil final est gelé sur ce point ; renderer/fixture/forward indépendants continuent.

L'auteur initial a terminé et été shutdown après intégration. Sa reprise via team_message n'a que mis un message en attente ; une demande native de continuation est restée en inbox sans nouveau résultat. Cette entrée créée par le lead a été annulée, sans interrompre une exécution ni modifier les droits. Le complément ci-dessous est une tâche distincte de décision, pas une seconde implémentation ou une réparation du plugin.

## Faits vérifiés

- Canon application : HEAD435376f puis attribution testsHive b20d6b8 ; SDK72845d9 publié sur branche de mission. ADR0314 Accepted / Unimplemented, ADR0304 Accepted / Partial, ADR0305 Accepted / Implemented, ADR0308 et0412 Accepted / Partial,0606 Proposed / Partial. Les contrats ratifiés `authz-trust-domain-boundary.md` et `0412-oauth-local-full-alignment.md` sont les entrées.
- Serena canon : `services/IAMRusty/configuration/src/lib.rs::JwtConfig::platform_issuer`, lignes490–494, construit public_base_url sans slash final + /iam. `http_verifier_auth`,434–488, utilise cette valeur pour RS256 ; issuer HS256 peut rester distinct. Références inspectées : setup_jwt, tests setup/configuration et seeded_verifier_configuration.
- `services/IAMRusty/setup/src/app.rs::setup_jwt`,1286–1380 : utilise cette même autorité pour token_service, registry/binding et signatures ; récupère un AuthConfig neuf de http_verifier_auth, puis copie uniquement config.auth.mesh. Les futurs champs policy explicites doivent donc être brokerisés par le lead, pas supposés déjà consommés.
- Nouveaux champs SDK vérifiés : `[auth.jwt] platform_issuer` optionString et `allow_historical_platform_issuer` optionBool. Les deux doivent être explicitement présents pour les composition roots normaux. Pas de plateforme inférée du Host, HMAC, HS issuer ou JWKS. Le champ historique ne devient pas true par défaut.
- Le plan laboratoire propose public_base_url18080 pour les callbacks exacts, vrai GitLabConnect HTTPS/HMAC, IdP simulé, autorité supposée8080. Le lead a gelé les valeurs finales plutôt que coder cette incohérence.

## Décision demandée à architecte

Écrire une proposition courte `docs/contracts/0412-oauth-proof-authority-alignment.md`, statut Proposed uniquement. Comparer et recommander le plus petit assemblage cohérent avec l'existant : coordonner l'autorité effective du profil de preuve à18080 et ses lecteurs/producteurs si conforme, plutôt qu'ajouter une API ou un stockage produit séparant origine et autorité. Vérifier les seeds/JWKS metadata, IAM, ext-authz et les consommateurs réellement rendus. Si une autre option est nécessaire, justifier par une contradiction concrète d'ADR/source.

Fournir les snippets ou valeurs exactes, les profils/fichiers à aligner, la fenêtre historique explicite, les invariants sans fallback et les tests discriminants nommés avant run. Pas de nouvelle ADR par principe ni Accept d'ADR0606. Pas de reset, rotation de clé/binding, migration runtime ou changement de plateforme automatique ; profil neuf sous bail futur seulement.

Ownership : seul nouveau contrat proposé, aucun index/ledger/autre contrat ratifié modifié par l'auteur. Le lead examinera, ratifiera et intégrera avant de libérer les valeurs finales au consommateur. Lecture seule des sources ciblées, écriture documentaire après transition native ; aucun code applicatif/SDK, test, Cargo, cluster, navigateur ou ressource Docker/Kind/WSL. Préserver toute modification étrangère et les anciens paquets de proposition présents dans un worktree réutilisé.

Le complément est dans la délégation longrun approuvée : pas de nouvelle question utilisateur pour cette décision nécessaire, modèles/QA/permissions inchangés. Pas d'enquête ou développement Ensemble au-delà du constat de livraison ci-dessus.
