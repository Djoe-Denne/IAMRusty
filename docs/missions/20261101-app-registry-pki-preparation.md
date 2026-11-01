# Préparation B — PKI du registre applicatif et pins de transport

2026-11-01, suite de [A](20261101-local-image-builds.md), publiée en
`586fa49dac1febddc2a12a10035ef487f9f5cc0f`. Cette préparation ne clôture pas B.

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
