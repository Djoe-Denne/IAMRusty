# Atlas visuel IAM / OAuthConnect

## Commencer par la photographie actuelle — 8 novembre 2026

**[Ouvrir l'atlas HTML autonome](iam-oauthconnect-current.html)** ·
[Lire les cinq vues et leurs sources Mermaid](iam-oauthconnect-current.md)

L'atlas suit cinq questions, dans cet ordre :

1. **Architecture et confiance** : qui dialogue avec qui ? Où sont les comptes,
   transactions, profils et clés publiques ?
2. **Cinq familles de matériaux** : qui garde le privé ou le secret, qui utilise
   le public, qui peut renouveler quoi ?
3. **Deux rotations différentes** : le fournisseur OIDC change ses clés ;
   Connect renouvelle son cache public. IAM a son propre cycle de signature,
   demandé via Hive et orchestré avec Transit dans le chemin inspecté.
4. **TLS, HMAC et signature** : quel hop est protégé, quel certificat est vérifié,
   quelle preuve reste nécessaire après le transport ?
5. **Nonce et PKCE** : pourquoi IAM persiste avant authorize, puis commit la
   consommation et la double purge avant complete ?

### À lire avec les dessins

- [Guide pédagogique complet](../guides/iam-oauthconnect-securite-clefs-jwks-tls.md)
  : définitions, fonctionnement, FAQ et diagnostic raisonné.
- [Dossier de sources](../guides/iam-oauthconnect-securite-sources.md)
  : commit de référence, symboles/lignes, standards primaires, preuves et inconnus.
- [ADR0413](../adr/0413-oauthconnect-multi-instances-profils.md) :
  **Accepted / Partial**. Les diagrammes expliquent cette architecture ; ils ne
  créent pas une nouvelle décision, une autorisation de rotation ou une certification live.

## Bien lire le statut

Le service Connect et le raccord IAM v2 sont livrés et qualifiés **localement
sur leurs périmètres respectifs** : 295 tests Connect et 325 unités + 80 IT IAM
rapportés dans les missions antérieures. Ces résultats ne sont pas réexécutés
pour l'atlas et ne s'additionnent pas en preuve globale du parcours.

Restent notamment non qualifiés : chaîne intégrée G5 IAM→Connect réel→vendor
signé, fournisseurs réels, équivalence GitLab `sub`/ancien identifiant, bascule
et retrait legacy. Les configurations et certificats réellement déployés ne sont
pas observés ici. Un cadenas ou une flèche ne signifie donc pas « production vérifiée ».

## Consultation hors ligne et impression

- Copier le dossier `docs/` en conservant son arborescence pour garder aussi les
  liens vers les guides et contrats. L'HTML seul reste lisible : ses cinq SVG,
  sa palette et sa mise en page sont intégrés au fichier.
- Ouvrir `iam-oauthconnect-current.html` directement, sans serveur. Aucun script,
  CDN, police distante, image distante ou acquisition réseau n'est nécessaire.
- Les liens sont des liens de navigation explicites ; ils ne sont pas des
  ressources chargées au démarrage. Les références externes sont dans le dossier
  de sources, pas des dépendances de rendu de l'HTML.
- Sur petit écran, chaque vue conserve sa largeur minimale et se parcourt par
  défilement horizontal. Le texte explicatif sous les dessins reste adaptable.
- Pour un PDF : **Ctrl+P → A4 paysage**, marges du document, désactiver les
  en-têtes/pieds du navigateur. Une vue par page est prévue en CSS ; vérifier
  l'aperçu, l'échelle et les coupures sur le navigateur utilisé. Aucun PDF ni
  résultat de pagination universelle n'est promis par ces sources.
- La version Markdown donne une alternative textuelle complète. Ses Mermaid
  sont des sources maintenables ; leur rendu dépend du lecteur Markdown et n'est
  pas nécessaire pour consulter l'HTML.

## Photographies historiques à préserver — 7 novembre 2026

- **Architecture cible** : [HTML original](oauthconnect-architecture.html) ·
  [notice historique](oauthconnect-architecture.md).
- **Clés, rotation et responsabilités** : [HTML original](oidc-keys-responsibilities.html) ·
  [notice historique](oidc-keys-responsibilities.md).

Ces deux formes SVG/HTML appréciées sont conservées. Un bandeau rappelle qu'elles
précèdent la fin du lot service Connect et le raccord IAM. Leurs textes « en
cours », « refusant » ou « hors lot » décrivent leur date, pas les sources actuelles.
Les Canvas d'origine étaient locaux au poste de création ; ils ne sont pas requis
et aucun lien absolu vers un poste particulier n'est nécessaire à la lecture.

## Maintenir cet atlas sans perdre les distinctions

1. Relire le [dossier de sources](../guides/iam-oauthconnect-securite-sources.md)
   et le code concerné si le comportement a changé ; dater une nouvelle photographie.
2. Modifier les Mermaid et leurs explications dans
   [iam-oauthconnect-current.md](iam-oauthconnect-current.md), puis le SVG statique
   correspondant dans l'HTML. L'HTML n'exécute pas de conversion Mermaid au chargement.
3. Conserver les couleurs de rôle : IAM bleu, Connect vert-bleu, fournisseur violet,
   opérations et précautions ambre. Les rôles sont aussi écrits : la couleur ne
   doit jamais être le seul moyen de comprendre une responsabilité.
4. Conserver les cinq familles distinctes, le callback sur IAM, le commit avant
   complete et la différence entre issuer et `kid`.
5. Ne pas transformer TTL300s/cooldown30s en calendrier de rotation privée, ni
   le TTL IAM+60s en délai du fournisseur ou durée d'un certificat.
6. Ne pas attribuer Rotate au remote signer : le chemin automatique inspecté
   de fabrication du successeur est Transit. Une capacité Sign ne prouve pas Rotate.
7. Contrôler chaque texte dans sa boîte, les espacements de lignes, les routes
   des flèches et les ancres ; vérifier ensuite le rendu navigateur et l'impression.
8. Ne jamais introduire de JWT, code, verifier, nonce, secret ou privé réel dans
   un exemple. Aucun état live, calendrier Ops ou fournisseur de CA ne se déduit
   du seul diagramme.

La liste des fichiers et les contrôles réellement effectués sur cette photographie
sont consignés dans la [note de fabrication de l'atlas](iam-oauthconnect-current.md#fabrication).
