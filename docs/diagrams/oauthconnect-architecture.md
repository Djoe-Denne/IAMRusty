# OAuthConnect — schéma d’architecture cible

> **Archive historique, avant la fin du lot service Connect et le raccord IAM.**
> Pour les faits courants du8novembre : [atlas actuel](iam-oauthconnect-current.md)
> et [HTML autonome](iam-oauthconnect-current.html). Les mentions « en cours » et
> « hors lot » ci-dessous conservent leur sens au7novembre, pas le statut actuel.

Photographie pédagogique du **7 novembre 2026**, exportée depuis un Canvas original
local au poste de création. Cette provenance ne constitue pas une dépendance de
consultation : aucun chemin propre à une machine n'est nécessaire.
Les blocs, flèches et textes du schéma sont conservés dans
l’[HTML autonome](oauthconnect-architecture.html), avec SVG et CSS intégrés.

## Ouvrir et imprimer

- Ouvrir `oauthconnect-architecture.html` directement dans un navigateur local
  (double-clic ou **Ouvrir un fichier**). Aucun serveur, JavaScript, React,
  dépendance ou accès réseau n’est requis ; aucune ressource externe n’est chargée.
- Pour un PDF : **Ctrl+P → Enregistrer au format PDF**, papier **A4**, orientation
  **paysage**, échelle **100 %** ou **Ajuster à la page** selon le navigateur.
  Désactiver les en-têtes/pieds de page du navigateur ; les couleurs d’arrière-plan
  sont facultatives. La feuille de style d’impression adapte les espacements.
- Sur petit écran, le diagramme conserve sa lisibilité par défilement horizontal ;
  les trois états passent en colonne. L’impression conserve les trois colonnes.

L’HTML est le livrable versionné visualisable. Aucun PDF/JPEG n’a été généré.
La structure HTML/SVG et les liens locaux ont été contrôlés ; le rendu visuel et
la pagination PDF **n’ont pas été vérifiés**, le navigateur MCP étant déconnecté.

## Portée de la photographie

Ce document conserve l’état du schéma apprécié, pas un statut live ni une nouvelle
décision d’architecture. Il décrit la **cible** : navigateur via IAM, échanges backend
S2S HMAC, un service OAuthConnect configurable, registry immuable d’instances et
deux profils OAuth/OIDC. `github` et `gitlab` sont des exemples de slugs, pas des
types codés en dur. IAM reste propriétaire des UUID/comptes et des transactions.

- **Acquis** : configuration, mapping OAuth, HTTP/secrets, contrat v2 et 189 tests
  du lot précédent ; ce chiffre ne constitue pas une preuve OIDC/service.
- **En cours au moment du schéma** : OIDC, cache technique JWKS et registry/service ;
  socle T0 avant preuves comportementales RED → GREEN, sans GREEN anticipé.
- **Hors lot** : adaptation IAM v2 et nonce persistant, E2E fournisseurs externes,
  bascule/déploiement. Aucun raccordement IAM v2 ni déploiement n’est annoncé.

Les anciens services restent pendant la transition. Les UUID IAM sont préservés,
sans migration de comptes ; l’équivalence `sub`/ancien ID GitLab reste une gate de
bascule. Connect n’a pas de cache de corrélation métier.

## Sources contractuelles

- [Contrat multi-instances](../contracts/oauthconnect-multi-instances.md), §§2.4, 3, 4 et 5.
- [Coutures OIDC, registry et service](../contracts/oauthconnect-oidc-service-boundary.md).
- [Frontière HTTP et secrets](../contracts/oauthconnect-http-secrets-boundary.md).
- Ledger du lot : `.cursor/handoffs/20261107-oauthconnect-oidc-service.md`, consulté
  dans le checkout canonique du dépôt ; document local
  évolutif, non embarqué dans l’export et distinct de cette photographie figée.
