# ADR-0607 : Déployer des images OCI par digest liées à leurs entrées, avec un Zot local indépendant de Kind

- Statut : Proposed
- Réalité : Partial — chemin images local codé par le parent ; registre OCI et pull Kind non qualifiés
- Date : 2026-11-01
- Décideurs : à renseigner après accord humain sur le texte final
- Jalon concerné : Ops portable / images locales et release, hors Apparatus P0–P6
- SuperSède : aucune
- SuperSédée par : —
- Contrat d'implémentation : [images-local-release-registry.md](../contracts/images-local-release-registry.md)

La direction utilisateur **Zot local, vitesse/cache, arbre courant en local, commit exact en release** est validée. Elle ne vaut pas acceptation globale de cette ADR, de ses mécanismes ni autorisation d'exécuter un runtime pendant sa préparation.

**Arbitrage humain explicite du 2026-11-01 — « Snapshot final » :** pour le local, accepter A→B→A si la compilation utilise la copie immuable A et si les entrées finales sont byte-identiques à A, SDK compris ; refuser toute différence finale. Oracle versionné **`snapshot-final-v1`**. Il n'atteste pas la détection des modifications transitoires et n'exige ni watcher natif ni daemon. Cet arbitrage seul est accepté, pas l'ADR entière ni une qualification cloud.

## Contexte

Base de mission : `master@f60910589e3867236c71beacb3ddbce447db74f7`, SDK consommé `93b890f`. `Dockerfile.build` possède des caches Cargo Linux ; `.cargo/config.toml` fixe 24 jobs et un target **Windows** sur E:. Le target Linux est déjà séparé par `CARGO_TARGET_DIR`. Ce ne sont pas encore des garanties de fraîcheur du binaire : Cargo utilise notamment les mtimes, tandis que Docker ne prend pas le seul mtime en compte pour son cache COPY.

À la base `f609105`, `ops/deploy/local_full_image_provenance.py::verify` compare labels SHA/SDK HEAD et `git diff`, sans contrat d'instantané des nouveaux fichiers/SDK dirty. `local-full-images.py` épingle des **IDs Docker** sous des tags ; `deploy-local-full.py::verify_images` les inspecte puis appelle `LocalClusterTargetAnchor.kind_load`. Un ID de configuration Docker n'est pas le digest d'un manifeste OCI publié.

Delta parent reçu pendant cette rédaction : `ops/deploy/build/images.py` fournit plan/build portable, snapshot, profils et manifeste **`aiforall.image-build/v1`** portant explicitement IDs Docker et `freshness_policy=snapshot-final-v1` ; provenance content et `--build-manifest` raccordés aux consumers, compatibilité kind-load conservée. Après corrections B1/B2/R relues : **52 tests Python offline PASS (30.928 s)** et **un oracle Cargo réel natif Windows 1/1 PASS (2.473 s)** communiqués par le parent. Ce dernier couvre compilation de B/asset build.rs, échec partiel, rollback avec mtimes host restaurées et exécution finale **A:A**. Ce n'est pas une preuve Linux/BuildKit, de publication/pull OCI ou d'E2E ; l'architecte ne réexécute pas ces campagnes.

Zot P4 existant (`ops/deploy/p4/zot.yaml`, stockage adapté dans local-full) héberge les enveloppes Apparatus et dépend du cluster. L'utiliser comme premier registre d'images applicatives créerait une dépendance circulaire et une confusion d'ownership. Son image source est Zot **2.1.8** (`ops/deploy/p4/Dockerfile.zot`), pas 2.1.22. Le wiki P4 distingue déjà enveloppe OCI et image CRI ; sa preuve historique n'atteste pas cette nouvelle distribution.

## Décision

**Le déploiement consomme un manifeste d'images OCI `repository@sha256:…`, lié à des entrées identifiées ; la compilation et la publication restent des opérations explicites. Le registre applicatif local est un Zot hors Kind, distinct du Zot P4.**

1. **Deux origines, une recette.** Local : instantané figé des entrées effectives du working tree, nouveaux fichiers pertinents et SDK réellement consommé inclus. Release : export automatique d'un commit exact de `master`, de son gitlink SDK et des locks/outils épinglés, jamais des modifications locales. Même backend Python, profils distincts ; aucune obligation de second checkout manuel.
2. **Fraîcheur finale avant disponibilité.** Empreintes de contenu, copie figée compilée et comparaison finale des entrées effectives, SDK compris ; différence finale ⇒ refus, pas promotion d'un ancien résultat. Sous `snapshot-final-v1`, A→B→A est admissible si résultat issu de A et état final A ; aucun audit des transitoires prétendu. Les mtimes conservées/rollback ne doivent pas permettre à Cargo d'exporter un vieux binaire. Les ensembles d'entrées sont fermés et non secrets ; pas de hash de `.env`, credentials ou états privés.
3. **Caches préservés.** Conserver le builder Linux existant et ses caches ; compiler/base stable, artefacts exacts et images applicatives autonomes séparés. Profil `dev-image` rapide pour le fonctionnel ; `release` pour livraison/performance. Une compilation release froide utilise un namespace target neuf, sans purge du dev. Aucun binaire monté dans le runtime nominal, aucun target Windows partagé avec Linux.
4. **Distribution locale sécurisée et sans cycle.** Zot 2.1.8 épinglé par digest, stockage propre persistant, HTTPS/mTLS local, identités publication/lecture distinctes. Bootstrap PKI privé indépendant d'OpenBao/Kind. Un helper OCI Skopeo épinglé publie les exports du daemon vers Zot ; cela évite de changer globalement la confiance Docker Desktop ou d'abandonner les caches du builder par défaut. Pas de socket Docker dans le helper.
5. **Kind reste derrière son autorité.** Les opérations réseau/trust containerd sont fermées et passent par `local_full_target` : leasev2, IDs exacts, endpoint/mapping avant API GET et UID avant mutation. Les workloads utilisent les digests publiés ; ni import `kind load` caché dans ce nouveau nominal, ni `docker exec` libre sur des nœuds. Le registre P4, ses credentials et ses volumes ne sont ni adoptés ni migrés.

Les interfaces, le transport mTLS compatible 2.1.8, la séparation des caches et les gates de consommation sont normés par le contrat lié. Le découpage n'impose ni nouveau service applicatif Rust ni abstraction SDK.

## Conséquences

- Nouvelle frontière durable de distribution/identité et reçu OCI distinct du reçu de build : seuil ADR atteint, plage cloud/cluster, **0607 libre vérifiée** à la rédaction.
- Une image n'est pas « actuelle » parce que son tag ou son HEAD paraît correct. Le lancement local vérifie les entrées effectives de chaque cible ; une image inchangée peut être réutilisée si cette preuve correspond encore. Une release ne prétend pas inclure le dirty courant.
- Coûts bornés : copie de contexte et export local d'image, petit client OCI, PKI et stockage du registre. Pas de promesse de build bit-à-bit reproductible, de signature supply-chain, de HA ni de temps chiffré avant mesure.
- Les consommateurs historiques de tags/manifestes doivent être adaptés explicitement. Le full deploy qui quiesce/rebootstrappe le coffre n'est pas un raccourci de refresh applicatif ; l'incrémental doit préserver le bootstrap durci, SA, policies, seed et migrations.
- 0600 (cible cloud/digests), 0606 (lab), 0505/0506 et les décisions P4 restent inchangées ; aucune acceptation ou clôture production déduite.

## Alternatives écartées pour ce nominal

- HEAD seul ou refus de tout dirty : ne représente pas l'arbre local demandé ; `git diff` ne couvre pas à lui seul les nouveaux fichiers.
- Mount du binaire hôte, `latest`, tag `sha256-<Docker ID>` pris pour un digest OCI : couplage runtime/cache ou identité erronée.
- Réutiliser/adopter Zot P4 : cycle de bootstrap et données d'un autre périmètre.
- Registre HTTP/insecure global ou nouveau builder mTLS imposant un cache froid : éviter ces compromis ; garder les caches et isoler la publication dans le helper.
- Harbor/Artifactory, disque ext4 supposé accessible, wipe/prune pour résoudre le cache : hors besoin et hors autorisation.

## Non décidé ici

Fournisseur/identité cloud, signature et attestation de confiance externe, HA/rétention/GC, migration du stockage ext4, multi-architecture et campagne E2E métier globale. La première tranche peut livrer images/publication/consommation sans déclarer ces capacités terminées.

## Références et réalité

- Canon antérieur : [0600](0600-cloud-portable-opentofu-k8s-gitops.md), [0606](0606-local-full-kind-isole.md), [contrat runtime/validation](../contracts/ops-kind-runtime-validation-separation.md).
- Sources locales : chemins/symboles cités ci-dessus ; tests `ops/tests/test_openbao_local.py::test_stale_image_source_or_sdk_is_refused_without_running_docker`, `test_local_full_acceptance_source.py`. Catalogue/export exact en cours par le parent : `ops/deploy/image_catalog.py`, `ops/deploy/build/compile-artifacts.py`. Delta `build/images.py` et résultats offline ci-dessus communiqués par le parent, non réexécutés par l'architecte ; pas de preuve du nouveau flux registre.
- Wiki de conception : `obsidian/AI FOR ALL/projects/manifesto/concepts/apparatus-p4-operator.md`, frontières enveloppe/image ; ses anciennes mentions d'anonymous read ne priment pas sur le YAML actuel.
- Documentation officielle et bornes versionnées : liens dans le contrat. Zot 2.1.8 mTLS/CN confirmé au tag, sans emprunter la configuration d'identité 2.1.22.
- Revue de raccord puis correctifs relus, 2026-11-01 : voir §9 du contrat. **B1/B2 corrigés, R gated avant capture/Docker ; B3 clos par l'arbitrage humain snapshot-final. Publication du code de A sous limites locales explicites : aucun bloqueur architectural restant identifié sur ce périmètre relu.** Qualification Linux/BuildKit/registre et livraison release/cloud non acquises. Aucun correctif de code ni test exécuté par l'architecte.
- **Aucune exécution Docker/WSL/Kind/Cargo/API cluster, image construite/publiée, secret réel ou preuve runtime nouvelle dans cette rédaction.** Réalité Partial limitée au chemin images local codé communiqué ; distribution Zot applicative et consommation OCI restent à implémenter/qualifier. Index et pointeur wiki à intégrer par le parent, hors des deux fichiers autorisés ici ; digest Serena synchronisé séparément.
