# Réception QA — migration Ops déclarative

## Complément de contrat v1.5

La cible Agent native des snapshots est différée au pin OpenBao2.6.2 : le clone d'auto-auth perd DisableRedirects. Pour cette réception, conserver le client HTTP strict existant et ses refus, sans nouveau fallback ni bump/patch/proxy ; toutes les définitions publiques restent déclaratives. La confidentialité n'interdit pas le token déjà contractuel dans le TOML Lazaret ; elle interdit sa propagation à IAM/simulateur. Les tests Agent futurs ne sont pas des gates implicitement ajoutés à la migration actuelle. Sources et condition de levée : contrat§4.1. Cette précision n'atteste aucun runtime.

Date : 2026-11-09. Statut : **plan avant réception, aucune validation exécutée**.
HEADbase : `51996139ec31317aaef85e49ab3c9266591f3902` ; SDK baseline : `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`.

## Mandat et sources

- Migration complète Ops définissable vers Helm/Compose/Kustomize/Bake/just ; Python limité aux façades, validations, transferts privés et protocoles de sécurité non déclaratifs nécessaires.
- Sources : `docs/contracts/ops-local-full-declarative-iac.md` **v1.1**, notamment §§2–8 et clarification canonique du transport §5, et `.cursor/handoffs/20261109-ops-iac.md`. ADR0609 reste Proposed : ni ce plan ni le GO d'implémentation ne valent Accept humain ou permission de recréation.
- Plan de réception, pas revue du code des auteurs. Aucun fichier d'implémentation en cours n'a été lu. Les quelques repères de tests ci-dessous proviennent exclusivement de `git show`/`git grep` à HEADbase ; aucune conclusion sur leur version finale.
- Pas de scoring, ordre de priorité ni pipeline de rôles ajouté : les priorités appartiennent à l'utilisateur. Les identifiants G servent au suivi, pas au classement.
- Seule écriture : ce fichier. Aucun test/CI/rendu/build lancé ; aucun appel Docker/Kind/WSL, Cargo ou accès aux secrets. Les commandes suivantes sont proposées au parent, **pas exécutées par le planificateur**.

## Entrée de réception et mutualisation

Recevoir un candidat intégré identifié par commit/diff et SDK, avec inventaire des chemins déclaratifs, pins et commandes réellement livrées. Ne pas qualifier des arbres auteurs divergents comme un seul résultat. Vérifier le canon Accepted applicable ; remonter un conflit au parent plutôt qu'adapter silencieusement l'oracle.

Disposer d'inputs **synthétiques** publics/privés distincts, d'un espace de tentative jetable et des outils hôte épinglés. Aucun build n'est requis pour entrer dans cette réception Ops. Le ledger annonce Helm4.3.1 et ses checksums, et les versions client des autres outils ; ces annonces restent à rattacher aux exécutables utilisés par le run final. Outil manquant = NOT RUN.

Réutiliser les suites ciblées du contrat et leur isolation. Avant leur exécution, le parent confirme que leurs imports/helpers ne lancent pas de runtime réel ; les mutateurs et accès réseau sont remplacés par les fakes existants. Pas de nouveau moteur/harness. Un même rendu normalisé et un même journal de faux appels peuvent prouver plusieurs G : ne pas créer une suite par exigence, ni un produit cartésien de tous les scénarios négatifs par profil.

- Quatre profils exclusifs : `base`, `gitlab-proof`, `github-openbao`, `oauth-connect`. Tous les couples profil/phase **supportés** sont rendus/lintés ; une combinaison non supportée est refusée explicitement, non omise.
- Les profils partagent un oracle de structure ; les différences OAuth/réseau/seeds restent vérifiées dans leur profil. Pour les gardes communes, un cas représentatif suffit sauf différence réelle de chemin.
- Un journal d'appels ordonnés couvre succès, erreur avant effet, échec après effet et reprise. Le journal doit constater les appels interdits absents, pas seulement une exception.
- Budget indicatif, sans effort imposé : setup/fixtures/outils/golden 25 %, parité et parsers 25 %, scénarios ciblés de protocole 25 %, **buffer nommé 25 %** pour raccords d'intégration/outils/pins. Une campagne live éventuelle a son propre bail/budget. Si capacité insuffisante, soumettre la réduction au parent/utilisateur, sans consommer le buffer ni déclarer les checks omis couverts.

## Repères baseline et statut des preuves

Inventaire borné des chemins `ops/tests` et recherches exactes à HEADbase ; aucun historique exhaustif ou exploration Rust. Context Mode a traité les sorties ; PATH Git sandbox et échappement d'un script ont été corrigés, sans repli hors MCP.

Assertions baseline effectivement lues :

- `ops/tests/test_openbao_local.py:test_g4_seed_partial_cas_and_lost_ack_never_erase_committed_document` : exige `cas: 0`, simule conflit sur deuxième écriture et ACK perdu après première ; vérifie conservation du premier document, liste des écritures et absence des sentinelles dans erreurs/stdout/stderr. Oracle réutilisable pour G-04, mais ne prouve ni transaction à deux documents ni nouveau chemin Helm/just.
- `ops/tests/test_local_full_route_profiles.py:test_gitlab_profile_preserves_all_33_ids_paths_methods_and_bodies` : conserve les 33 identités/méthodes/corps et les seules différences de statuts listées. `test_gitlab_profile_has_one_declared_connector_and_keeps_github_disabled` lit une ancienne config : conserver son oracle métier, raccorder sa lecture au rendu standard (ne pas simplement supprimer le test).
- Repères localisés, sans audit complet des corps : `test_local_full_environment.py:test_g4_seed_foundations_require_ack_uid_and_original_lease`, `test_g4_second_epoch_cannot_complete_interrupted_attempt` ; `test_local_full_envoy_ingress.py:test_envoy_ingress_requires_measured_tunnel_of_exact_owned_cp`, `test_envoy_ingress_patch_ack_is_frozen_and_lost_ack_never_grants_rollback` ; `test_image_refresh.py:test_foreign_uid_or_old_image_refused_before_node_operations`. Statut **unassessed**, noms non assimilés à des preuves.

**Aucun G n'est COVERED par ces lectures** : aucun test exécuté sur le candidat. Les anciens tests de texte/source peuvent devenir périmés ; les déplacer vers le rendu parsé sans affaiblir leurs invariants. Un contrôle de fichier présent, un doublon de golden ou un grep de constructeur ne remplace pas la preuve comportementale. À la réception, chaque G reçoit fichier:symbole, commande/candidat, verdict et explication du lien, ou GAP-accept/GAP-defer avec décision humaine explicite.

### Actualisation v1.1 — données relayées par le parent, non réception finale

- Parent : recoupement des **11 entrées `targets.json` et du groupe `local-full`** avec `git show 5199613`. Contribution à l'oracle d'inventaire G-08/O-03 ; pas preuve des sélections/exporters effectifs, des groupes directs ou des refus de provenance.
- Parent : fetch Calico officiel de **325176 octets**, SHA-256 `b200cb3c2c7341717abe98e4acc80a13117bcd2c9c46ad46e8c209c5753f0b2f`, annoncé égal au vendor. Contribution à G-07/O-03 ; pas preuve de l'overlay, des gardes réseau ni du candidat intégré final.
- Revue client relayée : deux exigences Bake discriminantes ajoutées à G-08 (sélection explicite `artifacts` pour export Docker requis ; binaires/features des groupes directs et du connecteur isolé). **Non vérifiées par le planificateur** ; à prouver sur les invocations/rendus effectifs du candidat.
- Ces éléments sont des preuves rapportées à rattacher à leurs commandes/sources et à la révision intégrée, pas des checks réexécutés ici. G-07 et G-08 restent partiellement non démontrés, aucun statut COVERED intégral attribué.

## Registre borné des risques, scénarios et oracles

Toutes les obligations G-01 à G-09 sont **explicites**, issues du contrat. État initial : **GAP de preuve sur candidat**, pas finding de code. Chaque ligne regroupe nominal et négatif d'un même comportement. Pas de probabilité/impact chiffré imposé.

### G-01 — Parité complète et renderer unique (§§2,7,8)

- Risque : un nombre correct d'objets masque une modification de droits/config, ou une façade Python corrige silencieusement un rendu incomplet.
- Scénario nominal : rendre les quatre profils et leurs phases via Helm directement puis via façade ; comparer les mêmes objets et configurations à l'inventaire/golden synthétique **produit depuis HEADbase**, pas depuis le nouveau chart. Deux noms de release distincts ne changent pas les noms/selectors historiques. Second rendu identique = même état désiré après normalisation autorisée.
- Négatif : profils incompatibles, type/borne invalide, référence requise absente ; refus avant rendu/effet. Vérifier qu'un rendu direct suffit sans correcteur Python, `lookup`, post-renderer, plugin/hook, téléchargement ou connexion API/DNS.
- Oracle : identités GVK/namespace/nom, références et selectors ; configs TOML/HCL/Envoy parsées, routes/scopes/instances/callbacks ; RBAC/NP, volumes/PVC/probes/securityContext/ressources, SA/audiences/token-file/CA ; init/snapshot et sidecar natif `Always`, `enableServiceLinks: false`, TLS sauf exception `isolated_test`. Normalisation bornée à ordre/formatage/annotations renderer et variables explicitement listées ; toute autre différence a une disposition revue. Parseur réel existant quand disponible ; absence de validation HCL signalée, pas masquée par une comparaison de texte.
- Niveau/effort : Python + Helm client, effort principal de setup mutualisé. Raccorder `test_oauthconnect_ops.py`, `test_openbao_local.py`, `test_local_full_route_profiles.py`.

### G-02 — Déclaration extensible, retrait réel des constructeurs (§§3,6,7)

- Risque : déplacement cosmétique des listes vers un autre fichier impératif ; workload oublié aux attentes/reprises.
- Nominal discriminant : ajouter un workload synthétique **de catégorie existante**, uniquement dans déclarations/métadonnées ; obtenir rendu, ensembles de phase, attentes, répliques et consommateurs d'epoch via les mêmes façades/fakes, sans modifier Python ni le graphe des phases. Exclure ce workload des livrables finaux hors fixture.
- Négatif : métadonnée obligatoire de phase/reprise/exclusivité absente ou contradictoire ; refus explicite avant effet, pas omission silencieuse/fallback nominal par nom.
- Oracle : rendu standard + journal d'appels attendus pour le nouveau workload ; diff de l'expérience limité aux déclarations. Inventorier zéro caller runtime des constructeurs supprimés (`configure`, OAuth/proof, Pods auxiliaires, config/commandes registre, listes Bake, mutations CNI) ; examiner callers dynamiques et façades publiques. Mesurer au **même périmètre** Python runtime hors tests avant/après : ledger baseline 21 fichiers/7176 lignes/273 def, méthode à reproduire et fixer ; réduction nette et résidus justifiés, sans seuil arbitraire. Un simple renommage/déplacement ne satisfait pas l'oracle.
- Niveau/effort : statique ciblé + un scénario Python/Helm partagé ; pas de nouveau harness. Ne pas supprimer les bases Kustomize kind-mesh/P4 encore consommées.

### G-03 — Barrières interphases, quiescence et migrations (§3.1)

- Risque : writer actif pendant migration, échec ignoré, scale-out prématuré, appel direct de phase mutante contournant le verrou.
- Nominal : journal séquentiel préflight → cible/CNI → quiescence → fondations → dépendances/bootstrap/root révoqué → seeds/Jobs ACKés et Complete → première réplique disponible → état final/rollouts/reçu. Quiescence = seuls Deployments sélectionnés à zéro ; démarrage = une réplique par périmètre de schéma exclusif ; final = quatre services plateforme à deux, Lazaret à un et autres valeurs déclarées.
- Négatif mutualisé par points de coupure : préflight rouge, `--confirm-quiesce` absent sur existant, Pod writer encore vivant, Job actif incompatible/Failed/timeout, dépendance indisponible ou migration première réplique rouge ; **aucun appel mutateur successeur**. Tester crash après mutation, double invocation concurrente et invocation isolée sans contexte/prédécesseurs : refus/état incomplet honnête, pas nouvelle phase finale.
- Oracle : ordre et absence d'appels dans fakes, rendu des phases et périmètres d'exclusivité ; aucune dépendance/Secret modifiée par quiescence, aucune paire de writers du même schéma lancée simultanément, pas de `kubectl scale` nominal. Échec ne rollbacke pas DB/KV/PVC ; restauration éventuelle des seules répliques sous UID/barrière démontrée, sinon non-ready. La seule déclaration `replicas: 1` ne prouve pas la fin des anciens Pods.
- Niveau/effort : Python + graphe just et rendus déjà G-01 ; compléter les suites environnement/OpenBao, sans multiplier une suite par phase. Observation réelle distincte L-01.

### G-04 — Epoch, CAS, ACK et non-adoption (§§3.2,8)

- Risque : version supplémentaire à epoch saine, promotion d'une tentative incomplète, perte du premier document après CAS partiel, UID étranger adopté.
- Nominal : même epoch saine = zéro nouvelle version/Job/identité ; epoch nouvelle = dépendants déclarés, inputs validés, bootstrap/root révoqué, CAS create-only, ACK/UID et postconditions avant promotion.
- Négatif : 403/transport KO distinct d'absence, document divergent/détruit/supprimé, conflit deuxième CAS, premier ACK perdu, Job/namespace/UID remplacé, source privée changée et second epoch pendant `pending_epoch`. Stop sans effacer les effets déjà commis, sans promotion/second writer ni fallback par nom/PID.
- Oracle : état privé synthétique avant/après, journal versionné des écritures (`cas: 0`), références ACK et appels absents ; pas de fausse transaction des deux KV. Adapter les tests baseline cités, pas remplacer leur assert par un simple code retour.
- Niveau/effort : fakes Python existants, cas de reprise partagés avec G-03 ; preuve réelle séparée L-01.

### G-05 — Autorité, confidentialité, commandes historiques (§§2.2,3.2,7)

- Risque : invocation just/kubeconfig ambiante contourne les gardes, secret passe dans Helm/Compose/log/argv, `up` devient redeploy.
- Nominal : façades et flags historiques conservés ; `up` sain no-op, stop/status sur seuls IDs ACKés, `refresh` image-only sous CAS UID/image ; `preserve-platform-tls` n'applique/lit/répare pas les Secrets exclus. Invocation privée canonique sous verrou global.
- Négatif : cible/engine/endpoint/UID étranger, kubeconfig ou `.env` ambiant piégé, chemin/symlink/reparse/ACL invalide, input privé modifié, PKI CA/SAN/identité/validité incorrecte, refresh avec configuration/features/assets/migrations modifiés ou image/SDK périmés. Refus avant accès/mutation non autorisé.
- Oracle : sentinelles synthétiques suivies de leur source jusqu'au consommateur autorisé ; absentes stdout/stderr/erreurs/rendu/argv/environnement des outils, aucune lecture Secret live pour preuve/reconstruction ; compteurs mutateurs à zéro sur refus. Corps des enveloppes privés limités aux champs permis/UID du Job ACKé. Vérifier distinctions image/build/release-manifest.
- Niveau/effort : suites `test_local_full_target.py`, `test_local_full_tls.py`, `test_local_full_environment.py`, `test_image_refresh.py` ; mutualiser leurs fakes et les cas de secrets G-04/G-06.

### G-06 — Compose neuf, réemploi ancien et échec partiel (§5)

- Risque : adoption/recréation ou second writer sur volume existant ; création partielle non enregistrée ; ACL/PKI affaiblies malgré YAML valide.
- Nominal : `config --quiet` avec chemins/inputs synthétiques ; vérifier Zot2.1.22 **au digest baseline vérifié**, loopback/user10001/rootfs/capabilities, stockage/réseau distincts, mTLS/ACL publisher vs reader sans anonyme, auxiliaires épinglés sans socket, commandes suivies et dépendances completed/health. Faux état neuf : reçu/registre à chaque création, pré/postflight sur identités réelles, pas seulement labels.
- Négatif : port/nom occupé, drift image/config/réseau/volume/membres, reprise partiellement échouée, ACK perdu/creation opaque. Chaque effet connu tracé ; inconnu = UNKNOWN/ask jusqu'à résolution, jamais propriété déduite. Pas de création après refus, recréation/adoption/down/rm/prune implicite. `--no-recreate` n'excuse aucune dérive.
- Ancien registre : lire/publier/reprendre/arrêter les seuls IDs historiques ACKés ; **zéro création/adoption/recréation/reconfiguration du service registre, de son réseau ou de ses volumes persistants**, pas zéro création de tout auxiliaire. `up`/status/stop historiques restent sans création. Faux labels Compose/import par nom/second montage writer refusés. Une transition du registre n'est pas automatique : opération explicite distincte, port/réseau/stockage séparés, OCI/digests/nouveaux reçus avant bascule, ancien préservé.
- Transport historique nominal v1.1 : publier hermétiquement vers un état historique **sans métadonnées `compose`**, puis publier une seconde fois avec un helper Skopeo neuf par opération, sans nouvelle identité persistante. Modèle effectif `registry/publish.compose.yaml` ou équivalent commun : seul helper épinglé et réseau vérifié `external: true`, aucun service registre/volume persistant/réseau à créer ni préparation de volumes. Revalider ID↔nom du réseau, propriétés/membres, registre/montages/PKI/provenance avant effet ; `external: true` n'est pas une preuve d'ownership.
- Oracle confidentialité/helper : archive OCI, destination validée, scripts/policy/config et certificats **RO** ; seules sortie digest et TMPDIR **RW**, séparées dans la tentative privée neuve, jamais un `/work` global RW. Rootfs RO, utilisateur/capacités/restart bornés, aucun socket ni volume data/runtime Zot. Reçu transport distinct : préimage absente, ID/spec/image/montages/réseau vérifiés, création journalisée immédiatement, **ACK privé avant start**. L'état étendu pour le helper ne convertit pas l'ownership v1 ni n'exige d'anciens ACK/empreintes de création Compose inexistants.
- Négatifs transport : mauvais réseau/ID, membre étranger, nom/projet occupé, setup incomplet, dérive de déclaration/PKI, ACK perdu, helper déjà exécuté/replay ou pending incertain et digest distant divergent : STOP sans réparation/adoption ni mutation du service registre. Vérifier l'échec partiel à la création et après démarrage, le stop du **seul ID helper possédé même sur échec/timeout**, et l'échec de stop consigné. Pas de `--rm`, suppression automatique, `compose down` ou prune ; helper terminé conservé inventorié, y compris comme membre réseau.
- Oracle commun : documents Compose résolus, ACL publique, journal fakes + reçus/registre synthétiques après chaque création pertinente. Postimage historique : IDs/labels/config/PKI/montages persistants inchangés ; seuls membre helper tracé, reçus transport et **écritures OCI normales de publication** admis. Contrôler archives/digest/config distants, publisher read/create/update vs reader read-only et non-écrasement du reçu de sortie. Remplacer un test bloquant toute publication v1 par ces succès/refus, pas supprimer sa couverture. `config` ne prouve ni enforcement mTLS/ACL, ni persistance, ni comportement daemon : L-02 séparé.
- Niveau/effort : `test_local_registry.py` adapté et client Compose ; pas duplication de la suite opt-in `test_registry_runtime.py` dans les unités.

### G-07 — CNI épinglé et réseau non portable (§4)

- Risque : téléchargement transformé/pin modifié, adresse historique promue en défaut portable, exception autorisée perdue ou réseau élargi.
- Nominal : provenance/checksum Calico3.29.7 vérifiés, overlay Kustomize CIDR10.244.0.0/16/env attendu ; allow-api et ingress rendus à partir d'attestations synthétiques liées au bail/IDs/UID/préimage. Valeur inchangée = no-op.
- Négatif : checksum différent, CNI/topologie inconnu, attestation absente/périmée/venue d'une reconstruction, IPIP incohérent avec nœud possédé, remplacement UID/RV/spec ou ACK perdu. Refus du delta dangereux ; ni `/32` historique codé en dur ni élargissement `/16`/`0.0.0.0/0`. Exception existante non retirée silencieusement.
- Oracle : digest/provenance baseline indépendants du fichier livré, diff parsé CNI et NetworkPolicy ; patch uniquement sous tests UID/RV/spec puis postimage exacte. Les tests ingress baseline gardent leur autorité ; le rendu seul ne prouve pas l'IP source réellement observée.
- Niveau/effort : Kustomize offline + fakes de `test_local_full_envoy_ingress.py`/cible et tests CNI du lot, puis L-01 seulement sous bail.

### G-08 — Bake, provenance et absence de build implicite (§6)

- Risque : cible/feature oubliée, double inventaire Python/HCL divergent, tag historique accepté malgré SDK/binaire périmé.
- Nominal : `bake --print` et inventaire typé comparés aux cibles/groupes/Dockerfiles/arguments/features/outputs baseline ; wrapper délègue et vérifie les entrées/sorties/provenances synthétiques. Conserver `snapshot-final-v1`, gitlink, empreintes binaires et fraîcheur.
- Oracle sélection/export discriminant, issu de la revue client relayée : si le wrapper exige l'export Docker d'`artifacts`, **sélectionner explicitement `artifacts` dans l'invocation Bake** et vérifier sa présence parmi les cibles sélectionnées du JSON effectif avec l'exporter attendu. Un `output` Docker dans le HCL seul n'est pas suffisant : la dépendance implicite cache-only signalée ne démontre pas cet export. Confronter le journal de l'invocation wrapper au `--print` correspondant ; le cas sans sélection explicite doit être détecté comme insuffisant, pas accepté sur présence de la déclaration HCL.
- Oracle groupes directs discriminant : examiner séparément les invocations standard des groupes `workspace` et `openbao`, ainsi que du connecteur isolé, et vérifier transmission effective des **binaires et features attendus** depuis l'inventaire/baseline vers les arguments/contexte des cibles concernées. Ne pas déduire cette transmission du seul groupe `local-full` ni d'une branche Python spécifique qui compenserait une déclaration directe incomplète. Absence/mauvaise valeur de binaire ou feature attendue = oracle rouge. Ces deux exigences sont à vérifier, non des constats QA sur le code en cours.
- Négatif : source/SDK/archive/digest obsolète ou sortie manquante ; refus avant packaging/release. Render/up/deploy/refresh n'appellent aucun build ; caches/builders et séparation Windows/Linux préservés.
- Oracle : JSON Bake résolu des sélections réellement invoquées et journal faux appels, pas grep de mots HCL ni reconstruction des cibles depuis le nouveau wrapper. Le recoupement parent des 11 entrées et de `local-full` est une preuve partielle d'inventaire uniquement ; **G-08 ne devient pas COVERED intégralement** sans les oracles sélection/export, groupes directs et gardes de provenance. `--print` ne prouve ni export Docker réellement produit, ni succès de compilation ni contenu d'image ; pas de build ajouté à cette réception.
- Niveau/effort : `test_image_sources.py`/`test_image_refresh.py` existants + client Bake ; une comparaison d'inventaire mutualisée.

### G-09 — Pods/outils stockage (§6)

- Risque : restore écrit dans une destination existante ou perd intégrité/ownership en externalisant les specs.
- Nominal : rendu **séparé du nominal déployable** des Pods backup/restore, nom de tentative/namespace/claim/RO-RW validés ; faux transfert binaire intact et destination nouvelle.
- Négatif : mauvais UID/PVC/PV/source, archive existante, destination occupée ou flux incohérent ; refus avant écriture/lancement. Aucun Pod outil inclus accidentellement dans apply normal.
- Oracle : manifest parsé issu du chart, gardes et reçus fakes, octets synthétiques incluant NUL/non-UTF8/CRLF inchangés. Pas de lancement réel backup/restore pour la rédaction/réception statique.
- Niveau/effort : cas stockage dans suite Ops existante concernée ; noms de tests à rattacher par l'auteur, sans imposer un module/harness nouveau.

## Commandes proposées et frontières de preuve

À exécuter **plus tard par le parent**, depuis la racine du candidat intégré, chemins explicites et environnement contrôlé. Les variables ci-dessous représentent des paramètres à renseigner depuis les livrables ; elles ne constituent pas de nouveaux noms de fichiers prescrits. Aucune recette mutante n'est lancée par les checks client.

1. **Python hermétique (n'exécute pas Helm par magie)** : `python -m unittest discover -s ops/tests -p <fichier>` pour les dix fichiers §8.1 du contrat et les seuls tests stockage/graphe/CNI livrés nécessaires. Rendu Helm appelé réellement dans les tests d'intégration client concernés ; un mock de stdout Helm ne qualifie pas son schéma/templates.
2. **Helm client** : `helm lint ops/deploy/charts/local-full --strict -f <profil.yaml> -f <phase.yaml> -f <inputs-publics-synthetiques.yaml>` puis `helm template qa-a ops/deploy/charts/local-full --namespace <namespace-public-valide> -f <profil.yaml> -f <phase.yaml> -f <inputs-publics-synthetiques.yaml>` ; inclure l'éventuel fichier commun déclaré, dans l'ordre livré. Répéter pour les quatre profils et phases supportées, comparer aussi `qa-b` pour indépendance des noms. Jamais `install/upgrade`, API, Secret dans `-f/--set` ou validation serveur. Comparaison façade/direct incluse G-01.
3. **Kustomize client** : `kubectl kustomize <overlay-CNI-livre>` (ou binaire Kustomize épinglé équivalent), comparer provenance/CIDR/env/objets attendus. Ni apply ni contact cluster.
4. **Compose config seulement** : `docker compose --env-file <env-public-synthetique-explicite> -p <projet-synthetique> -f ops/deploy/registry/compose.yaml config --quiet` avec les profils auxiliaires livrés si nécessaire ; même check sur `ops/deploy/registry/publish.compose.yaml` ou déclaration commune équivalente du transport seul. Compléter avec inspection parsée de config résolue **uniquement synthétique** pour G-06, dont service helper unique/réseau external/RO-RW séparés et absence de ressources persistantes à créer. Interdire up/create/build/run/down dans ces checks client ; les créations/stops transport sont simulés dans les tests hermétiques. Pas d'accès daemon attendu, pas d'auto-chargement du `.env` du lab.
5. **Bake print seulement** : `docker buildx bake -f ops/deploy/build/docker-bake.hcl --print <groupes-livres>` ; inclure l'inventaire associé si la commande livrée le requiert. Capturer aussi la sélection effective du wrapper avec **`artifacts` explicite si export Docker requis**, et séparément les groupes directs `workspace`, `openbao` et le connecteur isolé, avec leurs inputs synthétiques binaires/features attendus (G-08). Pas de build/bootstrap builder/pull/chargement d'image ; `--print` reste une preuve de plan, pas d'export produit.
6. **Just parsing/listage** : `just --justfile justfile --list` et `just --justfile ops/deploy/local-full.just --list`. Dry-run des façades seulement après contrôle des évaluations à parse-time/backticks/shell : `just --justfile <fichier> --dry-run <entree> <arguments-synthetiques>`. Un dry-run n'est pas intrinsèquement une preuve d'absence d'effets à parse-time. Le graphe et les statuts natifs Windows se prouvent aussi par G-03, pas par le listing seul.
7. **Hygiène** : `git diff --check` ; relevé reproductible du périmètre Python runtime, résidus/callers et façades. Les checks CI finaux se rapportent au candidat réellement conservé, pas à des résultats des branches avant intégration.

Pour chaque classe : version/commande exacte, code retour, nombre et identité des profils/phases/tests concernés, synthèse des assertions, écarts et verdict PASS/FAIL/NOT RUN. Sorties volumineuses via Context Mode ; ne pas archiver de sentinelles privées comme si elles étaient des configurations publiques. Aucun PASS remplacé par « outil absent mais YAML semble correct ».

## Live distinct — non lancé et sans autorisation déduite

- **L-01 (G-03/04/05/07)** : bail du parent, cible admissible et inventaire contemporain avant toute API ; observer la chronologie réelle fin des Pods writers → bootstrap/root révoqué → migrations première réplique → disponibilité → scale-out. Échec injecté borne les effets et n'ouvre pas la phase finale ; deuxième invocation saine ne crée aucune identité/version/stockage ; reprise d'epoch et deltas réseau attestés, persistance stop/restart possédé. Calico/CoreDNS/nœuds réellement prêts. Un mock ne qualifie pas ces propriétés live.
- **L-02 (G-06)** : campagne registre explicitement autorisée, suite `test_registry_runtime.py` sous garde opt-in existante ; mTLS bonnes/mauvaises CA/identités, reader incapable d'écrire, publisher autorisé sur son scope, anonyme refusé, digests et persistance vérifiés, IDs/reçus avant/après. Si transport historique inclus, helper neuf ACKé avant start, stop de son seul ID même sur échec et postconditions persistantes v1.1. Aucun scénario ne donne permission de recréer/reconfigurer/adopter l'ancien registre/réseau/volumes ; les écritures OCI normales de publication sont permises, pas un contenu OCI immuable exigé.
- Charger preuve/lifecycle au moment de la campagne, tenir bail/registre Docker et stop possédé selon canon. Aucun runtime n'a été créé/arrêté par ce plan. Une cible incompatible est bloquée, jamais recréée par défaut. OAuth navigateur reste preuve E2E séparée : HTTPS/readiness ne la remplace pas.
- État actuel L-01/L-02 : **NOT RUN — hors mandat de rédaction**. Un vert offline ne permet pas d'annoncer ces garanties démontrées ni de réutiliser les preuves historiques0608 pour le nouveau chemin. Le parent distingue réception statique et réception opérationnelle restante.

## Oracles à fixer au raccord — décisions ouvertes, pas nouvelle spec

Le contrat fournit les comportements discriminants ; aucun nouveau moteur de tests n'est nécessaire. Il manque encore des références concrètes de preuve, pas une autorisation de choisir silencieusement la sémantique :

- **O-01, parent + auteur déclarations, avant comparaison** : emplacement/version du golden synthétique HEADbase, allowlist exacte de normalisation/variables, disposition des différences intentionnelles. Sans oracle baseline indépendant, G-01 reste GAP même si tous les rendus lintent.
- **O-02, parent + auteur phases, avant G-02/03/04** : mapping livré profil/phase, métadonnées des attentes/dépendants et périmètres d'exclusivité des writers par schéma. Le contrat exige leur exposition sans en nommer le format : vérifier le format livré, ne pas inventer de liste QA parallèle. Si ce périmètre n'est pas déterminable, l'absence de second writer ne peut être conclue ; escalade parent/architecte si décision structurelle.
- **O-03, parent + auteurs images/réseau/registre, avant parité** : rattacher au candidat intégré les preuves parent déjà rapportées (11 entrées `targets.json` + `local-full` recoupées à HEADbase ; Calico officiel 325176 octets, SHA-256 ci-dessus égal vendor), sans refaire par défaut ce recoupement. Compléter les sources indépendantes des digests exacts Zot/auxiliaires et les oracles Bake sélection/export/groupes directs G-08. La version seule ou le checksum calculé du nouveau fichier ne prouve pas la conservation du pin. Ces preuves partielles ne qualifient ni le réseau live ni G-08 intégralement.
- **O-04, parent, avant bilan** : méthode/périmètre reproductible des mesures runtime Python et inventaire des exceptions restantes ; durée/commande précise de campagne live éventuelle et condition de réception opérationnelle. Pas de seuil de réduction ou timeout nouveau imposé par QA.

## Sortie et retour au parent

Réception statique démontrée quand les G ont leurs preuves minimales positives/négatives sur le candidat intégré, les comparaisons standard/baseline et parsers sont verts, les anciens oracles sont conservés/raccordés, les différences et exceptions sont disposées, et `diff --check` est vert. Toute lacune garde un état explicite, avec décision de report/acceptation si applicable ; aucune présence de test ne suffit à COVERED. Les blocages de confidentialité/autorité ou un conflit Accepted sont remontés, pas convertis en suggestions facultatives.

Si une preuve obligatoire manque, un check échoue ou un outil n'est pas disponible : réception concernée **non démontrée** et condition précise de reprise, sans nouvelle boucle de review automatique. Revue indépendante seulement si convenue par le parent. Publication/commit/push/nettoyage et mise à jour du ledger appartiennent au parent ; ce plan ne les exécute pas. Le parent persiste ce fichier avant de l'envoyer comme référence à d'éventuels auteurs isolés.

Synthèse non classée des trois familles de risque à porter au retour : (a) parité déclarative réelle/extensibilité, (b) barrières writers/reprise CAS-ACK-UID, (c) non-adoption registre/réseau non portable. Gaps de preuve transversaux : O-01/O-02/O-03 ; runtime L-01/L-02 NOT RUN. Réévaluer le même fichier après intégration, changement de contrat ou incident, sans ouvrir un chantier global de stratégie de test.

Actualisation du même plan : v1.1§5 relu, G-06 et oracles liés corrigés pour distinguer ressources persistantes et transport éphémère ; G-08 enrichi des deux exigences client ; preuves partielles parent consignées. Aucun code auteur lu, test/CI/rendu/runtime exécuté ni nouveau moteur/priorité ajouté pour cette mise à jour.
