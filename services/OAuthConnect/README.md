# OAuthConnect

Service HTTP d'authentification fédérée : instances configurées avec profil OAuth ou OIDC, contrat v2 groupant tokens et identité normalisée. IAM conserve les transactions navigateur, comptes UUID, liens, emails et décisions métier.

Le [profil ops local-full `--oauth-connect`](../../ops/deploy/README.md#oauthconnect--github--fournisseur-simulé-entrée-https)
prépare GitHub et l'instance OAuth `simulated`, via OpenBao, derrière une entrée
navigateur HTTPS. Il reste distinct des profils legacy et n'est pas une preuve
de déploiement live ou d'E2E fournisseur.

## Comprendre avant de configurer

- [Guide pédagogique : clés, JWKS, rotation et TLS](../../docs/guides/iam-oauthconnect-securite-clefs-jwks-tls.md).
- [Atlas et schémas horsligne](../../docs/diagrams/README.md), avec sources versionnées et vues historiques des Canvas.
- [Sources code et normes du guide](../../docs/guides/iam-oauthconnect-securite-sources.md).
- [Ajouter une instance compatible](../IAMRusty/docs/PROVIDER_FACTORY_GUIDE.md), sans copier un nouveau service ou ajouter un client vendor dans IAM.

## Autorités et contrats

ADR0413 est **Accepted / Partial**, pas une autorisation de déploiement : [décision](../../docs/adr/0413-oauthconnect-multi-instances-profils.md), [contrat multi-instances](../../docs/contracts/oauthconnect-multi-instances.md), [frontière OIDC/service](../../docs/contracts/oauthconnect-oidc-service-boundary.md) et [raccord IAM](../../docs/contracts/iam-oauthconnect-v2.md).

Les routes S2S `/oauth-connect/{instance}/v2/authorize` et `/oauth-connect/{instance}/v2/complete` sont HMAC par instance ; un Bearer utilisateur ne remplace pas cette authentification. Connect n'est pas un émetteur JWT utilisateur. Le JWKS public du fournisseur OIDC sert à **vérifier** ses ID tokens ; la clé privée et sa rotation restent chez ce fournisseur, pas dans Connect.

Le cache public JWKS en RAM est technique, par instance/processus. Ce n'est pas un cache métier de corrélation authorize→complete, ni une procédure de rotation de clés privées. Le nonce attendu et le verifier PKCE viennent d'IAM après consommation commitée.

## Où lire le code ?

- `configuration/` : catalogue/profils et validation de configuration.
- `domain/` : comportements et contrats neutres.
- `infra/` : protocoles OAuth/OIDC, transports et cache public.
- `http/` : registry/routing v2 HMAC.
- `setup/` : résolution des secrets, HTTP de confiance et composition complète avant écoute.
- `service/` : binaire natif et chargement du contexte d'exécution.

Le [rapport du lot service](../../docs/missions/20261107-oauthconnect-oidc-service.md) documente295tests, dont jeux OIDC signés et vrai binaire Windows ; le [rapport IAM](../../docs/missions/20261108-iam-oauthconnect-v2-results.md) documente325unités+80IT du raccord. Ces preuves restent séparées d'une chaîne intégrée IAM→Connect réel→vendor signée et des E2E fournisseurs réels.

Aucun script opérationnel de rotation, secret réel, configuration de cutover ou retrait legacy n'est exécuté par ces guides. Les exemples et schémas n'attestent pas quels certificats, KMS ou pods servent actuellement un environnement.
