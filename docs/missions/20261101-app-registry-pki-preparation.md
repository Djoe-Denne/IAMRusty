# Préparation B — PKI du registre applicatif et pins de transport

2026-11-01, suite de [A](20261101-local-image-builds.md), publiée en
`586fa49dac1febddc2a12a10035ef487f9f5cc0f`. Préparation PKI publiée ensuite en
`afbccf96a79176ed04f27afa4a9188a6dd084970`. Les sections initiales ci-dessous
décrivent ce jalon historique ; la suite opérationnelle est consignée en fin de fiche.

## Livré

- `prepare-local-full-certs.py --registry` ajoute `extensions/registry/` sans
  remplacer les anciennes identités ni modifier les CA/compteurs. Serveur SAN
  DNS `aiforall-apps-registry`, IP `127.0.0.1` ; clients `registry-publisher`
  et `registry-reader`, EKU clientAuth uniquement. Vérification des paires,
  CN/purpose/SAN/validité, certificat leaf seul, ACL privées et chaîne mesh.
- Les bundles partiels/périmés sont refusés ; aucune réparation en place ou
  génération de clés implicite par un consumer. Pas de certificat du registre
  injecté dans les images applicatives, pas de clé CA transférée à Docker.
- `ops/deploy/registry/transport.lock.json` : Zot **2.1.8**, Skopeo **1.22.3**,
  manifestes **Linux/amd64** épinglés par digest. Résolution HTTPS publique et
  SHA256 du corps de chaque manifeste vérifiés le 2026-11-01 ; version Skopeo
  issue de son label OCI, **pas encore exécutée**. Le tag source `latest` est
  une trace de résolution, jamais la référence à utiliser au runtime.
- L'image Zot officielle déclare UID 0 : le lock prévoit l'override non-root
  `10001:10001`. Le futur helper devra préparer uniquement ses volumes neufs
  ou possédés ; ce comportement n'est pas encore exécuté.

## Preuve Windows exécutée

Test ciblé `test_certificate_extension_preserves_every_old_byte_and_rejects_partial` :
**1/1 PASS en 28,536 s**. PKI synthétique possédée seulement, hors Git ;
génération base puis extensions OpenBao/GitHub + registre, contrôle de
préservation de chaque ancien octet, réutilisation idempotente, refus de clé
manquante dans chaque extension. Assertions sans exposer les clés ni leurs
empreintes ; un diagnostic déjà existant est vérifié append-only.

Deux rouges intermédiaires corrigés : représentation OpenSSL `IP Address:`
normalisée comme dans le validateur existant, puis oracle de journal adapté
à l'append du diagnostic privé après le second échec intentionnel. Les
certificats/clé/CA doivent toujours rester exactement inchangés.

## Limites et suite

**Aucun** daemon Docker/Kind/WSL démarré, conteneur/image/volume créé, credential
personnel lu ou registre P4 modifié. Les certificats synthétiques du test sont
retirés par le fixture ; aucun bootstrap root OpenBao ni seed n'est rejoué.

B restant : état/ownership privés, réseau/volumes propres, lancement Zot
non-root et readiness mTLS, export par ID puis copie Skopeo, relecture OCI et
reçu `aiforall.image-release/v1`. C (trust/pull Kind), D (refresh ciblé) et
release/cloud restent séparés et non qualifiés. Voir le
[contrat Proposed / Partial](../contracts/images-local-release-registry.md).

Références primaires vérifiées :
[Zot 2.1.8](https://github.com/project-zot/zot/tree/v2.1.8),
[Skopeo copy](https://github.com/containers/skopeo/blob/v1.19.0/docs/skopeo-copy.1.md),
[cert-dir](https://github.com/containers/image/blob/main/docs/containers-certs.d.5.md).

## Suite opérationnelle B/C/D — même journée

**Livré :** `local_registry.py` lifecycle/publish/vérification OCI, état privé lié
à l'engine et aux IDs exacts ; autorité Kind reader/trust/pull et consumers digest ;
action distincte `deploy-local-full.py refresh`, sans full deploy/provisioning.
Commandes et limites : [guide](../guides/linux-image-builds.md).

- Correction empirique de la proposition initiale : Zot **2.1.8** refuse les ACL
  avec mTLS seul. Application Zot **2.1.22** épinglé ; **P4 reste 2.1.8**, sans
  migration. Pas de htpasswd vide/proxy ([issue 3085](https://github.com/project-zot/zot/issues/3085)).
  `.Id` Docker est opaque : ce store containerd expose un digest manifeste,
  non un digest config. Pin contrôlé par RepoDigests + plateforme.
- **Runtime Zot : 1/1 PASS, 33,920 s.** Publisher push OCI, reader GET 200 et PUT
  403, même digest après stop/start du même registre avec stockage conservé.
- **Application réelle :** build Linux `ext-authz` release PASS ; second build
  inchangé **3,885 s**, même ID `sha256:853b200cc5ae3c54e8a501e1771a9099b5c9d22bb25819a1cadc62a051cc90b3`.
  Publication/relecture manifeste, config, filesystem et binaire PASS :
  `aiforall-apps-registry:5000/apps/ext-authz@sha256:dd015871d73913647e8515be64a32d44dc9de074f6f4f8a8894158778a9ff008`.
  Binaire `sha256:48118f146fe3b21b18f6adc24c52c81389fb3878b60e40030856691ea2a3b1d3`.
  Republication après ajout garde refresh : même digest, verify_mapping PASS,
  reçu `.tmp/ext-authz-release-refresh-baseline.json`, sans rebuild.
- **14 checks ciblés offline PASS**, incluant refus endpoint/UID avant node,
  reader par stdin sans CA key, stale source/SDK, patch image-only CAS,
  refus UID/image étrangers et config/migration, no-op sans rollout.
  Syntaxe 10 fichiers Python et `git diff --check` PASS. Pas de campagne générale.
- **Pull CRI / refresh live NOT RUN** : aucun bail v2 ancré sélectionné pour cette
  tâche. Le candidat historique j3 local est `pending_creation`, sans nodes ;
  les baux k6 sont v1/Cargo, pas des autorisations Kind. Aucune adoption/recréation
  ni aucun restart des nodes existants. Les services à migrations automatiques
  sont refusés par la V1 du refresh ; pas de contournement d'entrypoint.
- **Cleanup vérifié : 8/8 conteneurs possédés arrêtés, 0 running**, IDs/owners
  recoupés avec les registres Docker des 31 octobre / 1 novembre. Les 4 conteneurs
  préexistants sont inchangés (1 local running, 3 local-full arrêtés), aucun nouveau
  conteneur non possédé. 4 volumes, PKI, images et caches conservés, aucun prune/rm.
  Desktop/WSL partagés restent actifs ; RAM non mesurée. Pas d'équipe/branche/worktree créé.

La distribution locale est prouvée, **pas le bout-en-bout Kind ni une release
cloud**. Export exact master/gitlink et pins cloud restent gated ; preuve réelle
OpenBao/OAuth/données reportée à la campagne E2E finale. ADR0607 toujours Proposed.
