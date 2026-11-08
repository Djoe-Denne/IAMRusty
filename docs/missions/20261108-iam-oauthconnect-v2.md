# Raccord IAM → OAuthConnect v2

## Mandat et périmètre

Raccorder LOGIN, LINK et RELINK aux opérations v2 du service OAuthConnect déjà livré, sans bascule de déploiement. Conserver UUID, comptes, identités externes et liens existants. Réutiliser RustyCog et son pin ; aucun second harness E2E fournisseur.

Base : `e8952d805e81e49a940ef6d999bb74776f1d3166`. Cible : ADR0413 Accepted/Partial et [contrat §§5–7](../contracts/oauthconnect-multi-instances.md#5-adaptation-iam-et-contexte-oidc). **Travail en cours, aucune preuve IAM nouvelle de compilation/test à ce stade.**

## Vérification convenue

Exécution directe, sans mission Ensemble ni PM. TDD RED→GREEN et QA proportionnée : large couverture des parcours, vrais tests Postgres par fixtures RustyCog, HTTP sortant simulé, revues ciblées. Compilation et tests natifs Windows, un seul Cargo à la fois, 24 jobs.

Les quelques cas rares/ambigus encore rouges seront présentés **en un bilan fonctionnel groupé** avant toute décision de suppression ou réparation élargissant le comportement. Échec d'environnement, test absent ou compilation impossible ne valent pas RED comportemental ni PASS.

## Invariants et limites

Nonce IAM indépendant et durable avant START ; writer consomme puis efface nonce/verifier atomiquement avant tout complete. Migration additive nullable, refus des anciennes transactions sans nonce. Pas de fallback v1, profil dérivé du seul access token ou effet IAM lors d'un échec OIDC.

E2E GitHub/GitLab réels et déploiement restent à l'autre propriétaire. L'équivalence exacte `sub` GitLab/ancien ID reste une condition externe de bascule ; aucune migration de comptes ou réassociation implicite n'est autorisée.
