# Authentification — conclusions durables des revues finales

Cette synthèse publie les conclusions utiles de restitutions indépendantes du
29 octobre 2026 auparavant conservées dans les briefings locaux. Elle ne constitue
ni une nouvelle revue, ni une nouvelle exécution de tests. Le verdict canonique
reste dans le [contrat d'état qualifié](../contracts/20261029-auth-security-qualified-state.md).

## Autorité des tokens OAuth

La revue statique de sécurité a examiné le constructeur à quatre paramètres,
les deux constructions production et les fixtures concernées. Issuer et audience
sont obligatoires, privés et immuables ; construction des claims, encodage,
validations JWKS et durée restent dans leur contrat existant. Aucun défaut de
sécurité démontré dans ce delta, après passes défensive et adversariale.

Abus examinés : autorité contrôlée par une entrée OAuth, fallback de signataire
ou résurrection de clé, rejeu consommé, faux succès et exposition de credentials.
Pas de réaudit global des caches, du stockage, du SDK ou de PKCE. Les 19 tests
domaine, 3 tests codec, check IAM et fmt ont été relus comme preuves du parent,
pas réexécutés par le reviewer. Les intégrations et le vrai navigateur étaient
encore ouverts à cette date de revue, puis exécutés séparément.

## Sélection des suites d'intégration

La revue du sélecteur a confirmé trois suites IAM exactes sur liste fermée,
sans regex, alias, arguments ou chemins libres ni fallback vers toute la suite
IAM. Variantes de casse, couples croisés et répétitions examinés. Verdict statique
favorable sur ce delta, pas PASS des tests d'intégration. Les trois suites
exécutées ultérieurement totalisent 17 PASS, 0 échec, 0 ignoré et 0 filtré selon
le contrat qualifié.

## Recevabilité des oracles navigateur

La revue indépendante a relu huit pièces autorisées et recalculé leurs deltas.
Verdict : recevables dans leur périmètre, aucun gap bloquant supplémentaire.

- Même run et incarnation Envoy, epoch 0 et uptime croissant de 49 secondes.
- IAM 12→16, ext-authz 3→7 ; quatre compteurs IdP 4→6 chacun.
- Connexion Chrome 200, callback exact et trois chemins négatifs 403 ; contexte
  isolé. Les erreurs SAN/connexion/timeout mesurées n'augmentent pas ; IAM5xx
  reste 1→1 : l'erreur historique n'est pas effacée.
- Conservation des quatre appels IAM nécessaires aux quatre succès, routes
  protégées et 403 rendent recevable l'inférence du refus avant IAM. Ce n'est
  ni une mesure isolée de chaque négatif ni une mesure SQL directe.
- Arguments et montages du Pod, identité/version/hash de configuration, absence
  de configuration dynamique et effets observés relient la configuration aux
  routes testées. Ce n'est pas un `config_dump` intégral ni une validation de
  toute la configuration Envoy.

Quatre compteurs 5xx absents et certains statuts administratifs INCONCLUSIVE sont
conservés. La revue n'exigeait pas une nouvelle campagne SQL ou navigateur par
principe ; elle ne promouvait aucune ADR à elle seule.

## Infrastructure finale

La revue documentaire a réutilisé les preuves SAN, restauration et liaison
d'image, puis vérifié les pièces du navigateur sur le laboratoire ancré. Aucun
nouvel écart bloquant constaté dans ces pièces ; pas PASS global infrastructure,
production ou deux répliques déployées.

Identité Envoy, owners et configuration statique concordants ; deltas recalculés.
Fermeture de Chrome et des forwards attestée par le parent, pas nouveau contrôle
OS indépendant du reviewer. L'arrêt final des trois nœuds possédés a été effectué
et vérifié ensuite par le parent : ne pas l'attribuer à la revue précédente.

## Traçabilité non secrète

Sources locales relues pour cette synthèse :

- `20261029T1533Z-security-oauth-domain-authority-3c2e7ea.md` ;
- `20261029T1603Z-security-oauth-it-three-suites-7b8c1f95.md` ;
- `20261029T1602Z-security-oauth-browser-oracles-7b8c1f95.md` ;
- `20261029-infra-auth-lab-final-7b8c1f95.md`.

Ces briefings restent des traces locales sous `.cursor/review-briefings/`. Leur
contenu durable ci-dessus est versionné sans sessions natives, secrets ou sorties
brutes. Application/image/navigateur : `7b8c1f95` ; sélection QA complémentaire :
`8606173f` ; SDK : `02e5db23aac6016aab062c3078696d5f7fd464ef`.

Hashes SHA-256 non secrets relevés par la revue navigateur :

- navigateur : `06ef76f678c57433ac3dcab276c8e0895c5fa9d3c8c1c5180965ab4509400a9d` ;
- corrélation : `820111dfcaaf141a5690c210304151ffac4ea31f540f76b892b30fbf80c840cb` ;
- admin avant : `46e93f8ccc7ca9472bc4729b9d9ac92e9f0b8a469ed0b910821cea57c2cf042e` ;
- admin après : `b0db1231458ec7589c576b3abe4e1b2947057d45381e5a4438ab39ab6fe92a41`.

Ces hashes identifient des pièces privées ; ils ne permettent pas à eux seuls de
les reconstruire, de les rendre publiques ou de répéter la campagne.
