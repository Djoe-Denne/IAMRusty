# Profil CI Ubuntu — préparation, pas encore une campagne exécutable

Le mandat porte sur un job `k6-e2e` autonome, parallèle aux tests. Le
[contrat Proposed](../../../../docs/contracts/k6-three-scenario-ubuntu-ci.md)
décrit le résultat attendu. Aucun succès des tests ci-dessous ne qualifie
Chromium/Linux ou le parcours E2E.

## Éléments livrés

- `evidence.py` : réception fermée des cinq résumés existants, 34 checks,
  codes natifs explicites, deltas OAuth et causalité/restauration SAN.
- `inputs.py`, `config/*.toml.in` : secrets neufs et configurations publiques
  fermées, sans import de configuration opérateur. Ces templates sont validés
  syntaxiquement ; leur chargement par les vrais services reste à prouver.
- `config/envoy.yaml` : profil navigateur TLS, upstreams mTLS vérifiés,
  ext-authz fail-closed, stripping principal et exemptions exactes OAuth/health.
- Extension opt-in `ciEnabled: true` du chart natif, namespace
  `aiforall-local-full` : alias IAM isolé et vrai workload ext-authz. Sans cette
  option, l'inventaire natif reste inchangé à dix objets. L'option ne constitue
  pas une autorisation d'appliquer ces objets au lab existant.
- `pki.py` : PKI neuve du run, clés de signature JWT et neuf identités TLS,
  dont bon/mauvais SAN sous la même CA ; aucun import de confiance système.

## Réutiliser les sorties des builds, sans relancer Cargo

La matrice `build` de `ci.yml` utilise Ubuntu24.04 et conserve les sorties
**dev/debug non instrumentées** pour IAM (migration comprise), OAuthConnect et
ext-authz. Ce sont les mêmes compilations que les jobs de build, pas celles
de la couverture. Aucun cache ou ancien binaire n'est utilisé comme fallback.
Les trois bundles sont nommés `k6-binaries-<role>-<run_id>-<attempt>` et conservés
sept jours, avec SHA256, provenance, flags et métadonnées ELF.

Le workflow **`k6-images.yml`** est indépendant et manuel. Lorsque ces trois jobs
build sont verts et leurs bundles disponibles, il peut tourner même si les IT
de la CI productrice ne sont pas terminées :

```bash
gh workflow run k6-images.yml --ref master \
  -f producer_run_id="$RUN_ID" \
  -f producer_attempt="$ATTEMPT" \
  -f target_sha="$TARGET_SHA"
```

Les trois variables désignent le run CI, son attempt exact et son SHA applicatif
complet. Aucun choix implicite de « latest ». PR/fork, autre workflow, autre
attempt, build incomplet, expiration, instrumentation LLVM ou corruption donnent
un refus, jamais une reconstruction automatique.

`artifacts.py` vérifie la provenance API et le digest de l'archive avant extraction.
`binaries.py` vérifie ensuite le manifeste fermé, chaque hash et les métadonnées
ELF. `package.py` utilise les Dockerfiles existants avec le runtime Ubuntu24
épinglé, normalise les chemins sans rebaptiser le profil réel en release, et
vérifie les dépendances des quatre exécutables via des probes `ldd -v` isolées
du réseau. Aucun service n'est démarré par cette vérification.

Le résultat est un artefact `k6-images-<run_id>-<attempt>` contenant uniquement
`images.tar.gz`, `provenance.json`, `cleanup.json` et `docker-resources.jsonl`. Il est lui aussi
conservé sept jours. Les probes sans état sont arrêtées et retirées par ID ;
aucun prune ou retrait d'image préexistante. Les images restent sur le runner
jetable et sont exportées en archive, sans push de registry ou secret opérateur.
La fermeture Python est doublée d'une étape indépendante `always()` : reçus
contemporains et IDs/labels recoupés, arrêt gracieux des seules probes possédées,
inventaire final. Un label sans reçu permet de détecter une fermeture incertaine,
jamais d'adopter la ressource. Conflit ou preuve incomplète bloque le succès ;
une annulation dure de la VM peut toujours empêcher la collecte finale.

La provenance distingue **applications T / SDK S / harness H**. H peut différer
de T pour corriger les tests sans reconstruire les applications, à condition que
la seam d'inventaire soit compatible. L'entrypoint IAM provient toujours de T.
Un vert de ce workflow prouve seulement packaging et résolution ABI, **pas E2E**.

## Validation hors cluster

Outils existants explicites : `AIFORALL_HELM` et `AIFORALL_K6`, sinon recherche
PATH (replis Windows existants conservés pour les tests locaux). Les outils ne
sont pas téléchargés par les tests. Les scripts CI devront vérifier les pins
avant de définir ces variables ; leur présence seule ne prouve pas la version.

```text
python -B -m unittest discover -s ops/tests/k6/tests -p "test_ci_*.py" -v
```

Les tests de rendu utilisent réellement Helm ; ceux du projecteur exécutent
réellement k6 sans requête réseau ni lancement Chromium. Un outil absent donne
un skip explicite, jamais une preuve de qualification.

## Restant avant raccord CI

Le parent Ubuntu `campaign.py`, le raccord de la PKI et confiance NSS privées, le bootstrap
Kind/backings/OpenFGA, la provenance des images du checkout, les reçus IDs/UID,
les forwards, la mutation/restauration SAN et le cleanup failure-safe restent à
implémenter et tester. Le job de packaging ne doit pas annoncer un E2E vert en
exécutant seulement les modules présents ici. Qualification finale : vrai run
GitHub des cinq phases, puis vérification de ses artifacts allowlistés et du
cleanup. Aucun workflow historique réactivé, aucun secret local à transférer.
