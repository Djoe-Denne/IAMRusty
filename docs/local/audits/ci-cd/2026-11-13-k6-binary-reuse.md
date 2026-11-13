# Réutilisation des binaires CI pour les images k6

- Skill : ci-cd, amélioration bornée du workflow existant.
- Base : `33fddc8`, SDK `93b890ff` inchangé ; aucun changement produit Rust ou
  instrumentation de couverture. Contrat complémentaire **Proposed**, pas Accept.
- Vérifications locales : 57 tests `test_ci_*.py`, 10 régressions natives, sans
  skip ; vrai projecteur k6, Helm et OpenSSL hors cluster. Pas de compilation,
  Docker/Kind/WSL local ou campagne E2E exécutée.

## Constats et traitement

- **P1 — sorties applicatives non sauvegardées.** La CI n'upload que LCOV.
  Capture désormais les outputs fermés de l'inventaire après les builds natifs
  IAM/OAuth/ext-authz existants ; IAM migration sélectionnée explicitement,
  OAuthConnect ajouté à la matrice. Profil `dev/debug` maintenu. Bundles sept
  jours avec manifestes/empreintes ; flags et marqueurs LLVM contrôlés.
- **P1 — ABI du runner non garantie dans Bookworm.** Build épinglé Ubuntu24,
  cache natif séparé par ABI, runtime Ubuntu24 par digest vérifié dans l'API
  officielle Docker Hub. Les Dockerfiles existants conservent leurs défauts
  hors profil CI et acceptent le runtime/OpenSSL en arguments. UID1000 réutilisé
  s'il existe, aucun utilisateur de base supprimé. Résolution ELF à vérifier
  réellement dans les images avant de conclure.
- **P1 — consommation cross-run ambiguë.** Nouveau workflow manuel lecture
  seule avec run/attempt/SHA explicites. API : même repository ID et head repo,
  push/master, workflow CI exact, trois jobs build verts de l'attempt demandé.
  Pas besoin d'attendre le verdict global CI. Exactement trois IDs d'artefacts
  nommés, non expirés ; digest API ZIP puis hashes internes avant usage. Aucun
  fallback vers cache, autre run, autre attempt ou reconstruction.

## Preuves et limites

Tests négatifs : PR/fork/autre workflow/SHA/attempt, jobs manquants/dupliqués/
rouges/actifs, expiration/digest absent/corruption, liens et chemins traversants,
profil/flags/LLVM, sortie migration manquante dans l'inventaire, ABI non résolue.
Témoin positif H≠T avec entrypoint issu de T, quatre exécutables et profil dev
préservé. Trois images et métadonnées allowlistées seulement ; registre Docker
contemporain des probes ; aucun PAT, secret lab ou permission de registry.

Revue architecte indépendante : alignement confirmé, puis écart de fermeture
sur annulation corrigé avant publication. `cleanup.py` et étape `always()`
indépendante lisent les reçus du run/attempt, recoupent IDs et labels sans adoption,
arrêtent les probes possédées et vérifient l'inventaire final. Huit tests ciblés
couvrent interruption, absence idempotente, conflit/ask, arrêt échoué et probe
non reçue. Cette dernière laisse un verdict incomplet, pas un faux zéro-fuite.
Pas de suppression d'image, prune ou destruction globale. `cleanup.json` et
registre allowlistés sont collectés avant publication des images ; échec de
cleanup empêche l'artefact de succès. Annulation dure = limite explicite.

**Validation réelle encore requise** : publication des bundles par les jobs CI,
dispatch du workflow manuel puis packaging/résolution des quatre ELF. Aucun
succès de ces tests hors cluster ne vaut preuve E2E ; parent Ubuntu, bootstrap,
confiance NSS, cinq phases health/OAuth/SAN et cleanup du cluster restent à finir.

### Première publication et refus consumer

La [CI productrice43415018528](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43415018528)
est entièrement verte sur `1ca9a4c83226da50adb33822233b488c98a4736b` et publie
les trois bundles attendus. Le premier dispatch indépendant
[43420077346](https://github.com/Djoe-Denne/IAMRusty/actions/runs/43420077346),
harness `f8e54798` distinct de T, admet la provenance et vérifie les archives,
mais refuse le manifeste IAM avant toute construction d'image :
`MANIFEST_BUILD_ARGS`. Cleanup indépendant PASS, zéro probe possédée.

Cause constatée dans le manifeste réel : `iam-http_server` est un package Cargo
existant, mais le validateur des arguments n'acceptait pas l'underscore. La
grammaire est corrigée pour reconnaître ce nom valide ; chemins, options et
métacaractères restent refusés, cas de régression ajouté. Aucune suppression de
contrôle de provenance, de hash ou d'instrumentation. Les mêmes bundles doivent
être rejoués avec le harness corrigé, sans compiler T à nouveau. Les deux ELF
IAM exigent effectivement GLIBC2.38/2.39 et OpenSSL3 ; pas de preuve ABI runtime
avant le packaging réel.

Sources externes :
- [download-artifact au SHA réutilisé](https://raw.githubusercontent.com/actions/download-artifact/d3f86a106a0bac45b974a628896c90dbdf5c8093/README.md),
  cross-run et perte des permissions, d'où le tar fermé.
- [upload-artifact au SHA réutilisé](https://raw.githubusercontent.com/actions/upload-artifact/ea165f8d65b6e75b540449e92b4886f43607fa02/README.md),
  rétention et artefacts immuables.
- [Métadonnées Ubuntu24 officielles](https://registry.hub.docker.com/v2/repositories/library/ubuntu/tags/24.04),
  digest `sha256:2198e4c65076e9cb0b3d695219df667c0e3b0072da64ca3c2648aca17dcecaf5`,
  relevé du 2026-11-13. Ce pin ne prouve pas seul la compatibilité ABI.
