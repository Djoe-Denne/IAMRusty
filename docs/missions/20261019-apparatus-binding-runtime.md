# Proposition de mission — Apparatus : runtime HTTP Knative isolé par binding

**PROPOSITION — non approuvée, non lancée.** La rédaction des ADR et de cet
ordre de mission est autorisée ; elle ne vaut ni ratification d’ADR-0012, ni GO
implémentation, ni autorisation de spike ou de runtime. Les permissions de la
mission IT précédente ne sont pas héritées. Aucun commit/push automatique.

La policy de cette nouvelle mission sera présentée avec
`/ensemble-mission config` ; son exécution attendra le
`/ensemble-mission start` utilisateur. Un démarrage autoriserait d’abord le lot
de cadrage, **pas le franchissement automatique des gates ci-dessous**.

## 1. Résultat produit recherché

Un administrateur active/configure un plugin pour un projet sans démarrer de
Pod plugin ni demander de nœud plugin. Une route privée Knative est
pré-déclarée à zéro. Le premier appel **autorisé en amont** déclenche son
exécution ; Knative porte l’attente HTTP bornée, la montée en charge et le
retour à zéro après inactivité.

Une release OCI immuable est réutilisable dans plusieurs installations, mais
chaque `(project_id, binding_id)` dispose de son propre Service Serving,
configuration, références de secrets et identités. Des replicas multiples
peuvent servir un binding ; aucune instance d’exécution n’est partagée entre
bindings par simple égalité de digest.

Chemin cible : **caller à définir → autorisation Lazaret → ingress Knative
privé → Activator/KPA si nécessaire → queue-proxy → plugin Ready**. Le chemin
chaud peut éviter l’Activator ; ses garanties doivent être équivalentes. Un
appel non autorisé ne doit ni réveiller un plugin ni atteindre son code.

## 2. Canon et base de travail

- Dépôt : `C:/Users/djden/source/repos/AIForAll`.
- Base observée le 2026-10-19 :
  `2760e02bc43f5149070aac6ace29f7c2d11f9aad`.
  À l’exécution, enregistrer HEAD, diff et état du sous-module, sans écraser
  les changements présents. Les documents de cette préparation ne sont pas
  encore une nouvelle base Git commitée.
- Canon primaire : [ADR-0012](../adr/0012-apparatus-http-knative-par-binding.md),
  **Proposed / Unimplemented** ; canon secondaire :
  [contrat runtime Knative v1](../contracts/apparatus-knative-runtime.md).
  Cette mission planifie leur réalisation conditionnelle, sans les ratifier.
- [0011](../adr/0011-apparatus-p4-pod-par-binding.md) est **Rejected** : son
  corps est historique, pas une autre cible à implémenter en parallèle.
- [0008](../adr/0008-apparatus-p4-k8s-isolation-outside-manifesto.md) reste
  **Accepted / Implemented** pour P4-core. La supersession partielle de sa
  gestion du runtime ne sera inscrite qu’à l’Accept explicite de 0012 ; ni
  réécriture de son closeout, ni passage global à `Superseded`.
- [0009](../adr/0009-gate-preprod-scale-on-demand-isolation-instance.md) reste
  **Proposed / Unimplemented** : fermeture seulement après 0012
  **Accepted + Implemented ET preuves propres 0009 revues**.
  [0010](../adr/0010-gate-preprod-workload-certificate-ca.md) reste ouverte,
  indépendante de ce mécanisme.
- [0600](../adr/0600-cloud-portable-opentofu-k8s-gitops.md) demeure
  **Proposed / Unimplemented**. Helm peut packager, Flux peut réconcilier ;
  cette mission ne décide ni livraison cloud ni installation de Flux.

Faits de code déjà établis, à ne re-vérifier que si le HEAD dérive :

- `workers/apparatus-operator/src/controller.rs` : `cr_name_for` et
  `pod_name_for` reposent sur le digest ; `apply_valid_record` et
  `reconcile_object` peuvent remplacer des objets. Une correction locale du
  nom du Pod ne fournit pas le chemin HTTP 0→1.
- `workers/apparatus-operator/src/desired_state.rs` : `ReadyBinding` contient
  projet, composant/binding et digest ; `list_ready` ne définit pas à lui seul
  générations, désactivation, retrait et fencing distribué.
- `services/Lazaret/application/src/invoke.rs` : locator par digest ; l’invoke
  actuel dépend d’une session de workload et de `CallOrigin::Background`.
  **Ce n’est pas déjà l’API end-user de réveil d’un plugin dormant.**
- `services/Lazaret/domain/src/identity.rs` distingue instance, binding,
  release, génération et grant revision. Aucun champ ne doit être supprimé
  pour adapter artificiellement ce modèle à un Service virtuel.

## 3. Invariants non négociables

1. **Standards plutôt que contrôleur de runtime maison.** Serving/KPA porte
   replicas, réveil HTTP, buffering et retour à zéro. L’adaptateur métier
   existant `apparatus-operator` projette/observe les Services par binding,
   sans gérer directement leurs Pods/Deployments enfants. Aucun deuxième
   proxy/queue HTTP, scheduler, HPA ou KEDA concurrent sur ces replicas.
2. **Activation logique distincte de l’exécution.** Pré-déclaration à zéro,
   sans requête de warm-up. Configurer et vérifier sur la version retenue :
   KPA, scale-to-zero activé, `min-scale=0`, `initial-scale=0` et
   `allow-zero-initial-scale=true`. L’échelle initiale upstream vaut 1 par
   défaut ; `min-scale=0` seul ne satisfait pas l’exigence.
3. **Autorité métier hors Kubernetes.** Manifesto garde activation,
   générations, consentement et révocation ; aucun client/token Kubernetes
   dans ses sources. Lazaret reçoit une projection de confiance, pas des
   credentials Kubernetes ni une URL du caller/publisher. Pas de fallback
   digest/local lorsqu’elle est absente, périmée ou ambiguë.
4. **Admission indépendante.** Adm-A reste seule source de `VALID` ; activer,
   router, rendre Ready ou installer par Helm ne crée ni `VALID` ni
   `VERIFIED`. Enveloppe OCI ≠ image CRI pinée. Les quatre responsabilités et
   identités P4 restent séparées ; le TCB Knative supplémentaire est recensé.
5. **Autorisation et confinement avant/après réveil.** Refus en amont, pas
   d’accès alternatif public ou intra-cluster au code auteur. « Cluster-local »
   et NetworkPolicy ne prouvent pas à eux seuls l’identité du caller. Les
   chemins froid/chaud et plugin → Lazaret doivent préserver l’autorité et
   l’isolation. Aucun bearer IAM remis au plugin.
6. **Bornes et propriété.** Max scale strictement positif, concurrence,
   ressources, taille/attente des requêtes, deadlines et retries explicitement
   bornés. Budgets cumulés lors de coexistence de Revisions pris en compte.
   Identités/UID et préconditions pour toute mutation destructive ; nom,
   digest, label ou field manager ne prouvent pas la propriété.
7. **Portabilité sans fausse promesse d’isolation/coût.** Pools système et
   plugins distincts ; plusieurs bindings par nœud plugin, pas un nœud par
   plugin. Les Pods d’un nœud partagent généralement son noyau : qualifier
   une sandbox ou limiter explicitement la confiance admise. Scale-to-zero
   plugin ≠ arrêt de la plateforme, suppression du stockage ou facture zéro.

## 4. Gate préalable — six blocages à lever, pas à contourner

Le **lot A** complète les B1–B6 du contrat. Pour chacun : décision écrite,
responsable, surfaces exactes modifiées, oracles de validation et ratification.
« Knative le fait » n’est pas une réponse suffisante.

- **B1 — caller et identité.** Définir qui appelle avant qu’une instance
  existe, sa preuve d’autorité, la cible binding/génération et le cycle
  d’enrollment. Prouver instance ↔ Pod UID ↔ Revision ↔ binding/release.
  Tout changement de route, DTO ou claims est inventorié et ratifié, pas
  ajouté implicitement au câblage de `/invoke`.
- **B2 — transport et isolation.** Choisir ingress/versions et établir les
  terminaisons TLS, pairs authentifiés, contrôles anti-headers forgés,
  NetworkPolicy/probes et profil de sandbox compatibles avec queue-proxy.
  Même démonstration à froid et à chaud ; accès direct aux anciens endpoints,
  Pods, Services, Revisions et URLs taguées interdit hors besoins plateforme
  strictement authentifiés. Sinon : STOP, pas d’affaiblissement silencieux.
- **B3 — projection/générations.** Définir source authoritative, writer,
  stockage/transport, CAS/fencing, fraîcheur maximale et comportement en panne.
  Distinguer génération métier, `metadata.generation`, Revision et grant
  revision. Inclure désactivation/retrait, replays et migration des objets
  historiques ; ne pas prétendre que `list_ready` suffit.
- **B4 — révocation, buffer et drain.** Fixer le point de linéarisation et le
  sort d’une demande révoquée pendant son attente Activator ou son exécution.
  Refuser immédiatement les nouvelles autorisations ne prouve pas l’arrêt
  d’un effet déjà engagé. Définir validation avant effet, epochs/identités,
  cancellation éventuelle et drain sans queue/proxy maison.
- **B5 — profil opératoire.** Épingler Serving/ingress et les dépendances ;
  arrêter des valeurs approuvées pour capacité, concurrence, ressources,
  deadlines de bout en bout, fenêtres d’inactivité et limites d’attente.
  Examiner les retries de toute la chaîne et le mapping `InvokeHttpError`.
  Aucun chiffre/SLO n’est inventé par cette proposition.
- **B6 — état et cycle métier.** Définir l’éligibilité HTTP au retour à zéro,
  état externe/persistance, migrations, références de secrets et politique de
  rotation/rétention/retrait. La mémoire et le filesystem éphémère peuvent
  disparaître ; pas d’hibernation mémoire ni de PVC implicite.

**Sortie A :** ADR/contrat et annexes amendés, compatibilités démontrées au
niveau autorisé, changements publics explicités et Accept humain de 0012.
Un spike nécessaire fait l’objet d’un périmètre et d’un accord séparés ; il ne
ferme ni choix métier ni gate pré-prod. Tant que cette sortie n’est pas
acquise, les lots de production restent **NO-GO**. L’approbation d’une
architecture ne vaut toujours pas autorisation de sa campagne runtime.

## 5. Lots conditionnels et responsabilités

La racine travaille directement si un seul agent suffit ; si une équipe est
autorisée, charger `opencode-ensemble`, puis choisir les membres selon les
lots réellement indépendants. Aucun agent n’est lancé par ce document.

### A — Finaliser le contrat et obtenir la ratification

- Propriétaire : `architecte`, lecture + documents uniquement.
- Périmètre : ADR-0012/contrat, annexe B1–B6 et inventaire des deltas API,
  événements, traits/DTO, schémas et migrations éventuels. Préserver les ADR
  et preuves historiques ; pas de modification de production.
- Livrable : choix réalisables et ratifiés, restrictions résiduelles,
  transition/rollback fail-closed et critères de passage de chaque lot.
- Validation : aucun B1–B6 laissé implicitement « à décider pendant le code » ;
  autorité humaine identifiée et Accept explicite consigné.

### Q — Plan QA avant implémentation

- Propriétaire : `qa-planner`, après intégration du contrat ratifié.
- Livrable : `.cursor/qa-plans/20261019-apparatus-knative-runtime.md`, risques
  impact × probabilité, registre obligation→test et critères entrée/sortie.
- Oracles : exigences du contrat et scénarios §6, contrôles positifs/négatifs,
  concurrence, révocation pendant buffer, ownership et régressions existantes.
  Ne pas transformer un échec de transport en preuve d’autorisation refusée.

### B — Socle Serving et profil réseau/isolation

- Propriétaire : lot infra explicitement attribué ; périmètre
  `ops/deploy/p4/` et fichiers de pile système nécessaires, à inventorier par A.
- Livrable : manifests/pins et profil approuvé, ingress privé, placement,
  ressources/quotas, réseau/probes et compatibilité sandbox/mTLS ; aucun chart
  publisher privilégié ni second writer Flux/adaptateur.
- Validation : rendu/configuration statiques d’abord ; preuve réelle seulement
  dans la campagne autorisée E. `infra-verifier` read-only après changement
  infra significatif. Installer la pile ne constitue pas un succès produit.

### C — Projection par binding et adaptation de l’operator

- Propriétaire : `hard-implementer`, seul écrivain du mécanisme sous
  `workers/apparatus-operator/` ; dépend des interfaces figées par A et Q.
- Livrable : Service stable par binding, Revisions de génération autorisée,
  apply idempotent/fencé et observation distincte de l’état logique ; retrait
  sûr. Admission par release non dupliquée artificiellement par installation.
- Migration : inventorier CR/Pods/Services digest et tous leurs consommateurs ;
  couper l’ancien chemin avant ouverture du nouveau, pas de double contrôle
  ni de suppression d’un objet partagé pour retirer un seul binding.
  Aucun chemin historique ne doit supprimer les enfants Serving.
- Validation : replays, conflits de writer/UID, génération retardée,
  désactivation puis événement ancien, redémarrage du contrôleur et retrait
  d’un binding sans affecter l’autre. Pas de lock mémoire vendu comme fencing
  distribué, pas d’adoption/force d’un objet de propriété inconnue.

### D — Caller, identité et routage Lazaret

- Propriétaire : `hard-implementer`, périmètre Lazaret et contrats consommateurs
  **exactement** attribués par A ; pas de client Kubernetes dans Lazaret ou
  Manifesto. Un fichier partagé avec C reçoit un seul propriétaire.
- Livrable : autorisation pré-réveil conforme B1/B4, projection/locator par
  binding et génération, transport/instance conforme B2, deadlines et erreurs
  conformes B5. Capacités plugin → Lazaret et autorité courante conservées.
- Validation : état périmé/indisponible = refus ; aucun endpoint dérivé du
  digest seul ni fallback local ; révisions/sessions obsolètes inutilisables.
  Un timeout après envoi ambigu ne déclenche pas de retry métier automatique.

### T — Tests durables et régressions

- Propriétaire : `test-engineer`, tests uniquement, selon Q. Les tests intégrés
  aux modules de production restent attribués à leur écrivain pour éviter les
  conflits. Bug produit révélé = preuve ROUGE, correction par C/D/B.
- Livrable : tests ciblés unitaires/IT et harness E2E servant les oracles §6 ;
  preuves de réseau réel, pas mocks Kubernetes comme preuve de confinement.
- B, C et D ne sont parallélisables qu’après gel des interfaces, avec worktrees
  isolés et ownership non recouvrant. T peut préparer des tests sur ces
  interfaces, pas inventer seul la sémantique manquante.

### E — Validation, reviews et clôture bornée

- Propriétaire : lead ; reviewers indépendants, contexte fresh, diff et contrat
  ratifié, sans conclusions des développeurs. Revues proportionnées :
  correctness, sécurité, couverture ; Rust/performance si surface concernée.
- Validation : gates Windows ciblées, IT finales puis E2E Kind autorisée,
  après corrections. Rapports PASS/FAIL/INCONCLUSIVE avec preuves, gaps
  explicites et critères de Q ; pas de validation redondante ni « tout cargo ».
- Livrables : rapport de résultats lié à cette mission, registre des preuves,
  inventaires avant/après, verdicts de review et limitations. `Implemented`
  de 0012 uniquement si sa portée est effectivement démontrée et revue.
  Revue séparée de 0009 ; 0010 et autres dettes restent ouvertes sans preuves
  propres. Aucun Accept ou changement de Réalité automatique par les agents.

## 6. Scénarios d’acceptation et preuves attendues

Chaque scénario lie préconditions, actions, oracle métier, observations et
artefacts horodatés aux versions/configurations réellement testées. Les délais,
plafonds, concurrence et fenêtres d’attente viennent de B5 ratifié ; sans ces
valeurs, le scénario n’est pas déclaré mesurable ni clos.

1. **Activation sans exécution.** Binding autorisé et release `VALID` : Service
   et route déclarés, aucun Pod plugin ni demande de nœud depuis l’activation
   jusqu’au premier invoke, y compris au-delà du délai de réconciliation et
   après redémarrage du contrôleur. Observer aussi les créations transitoires :
   un simple instantané final à zéro est insuffisant. Pas de warm-up caché.
2. **Refus avant réveil.** Caller non autorisé, binding désactivé, release non
   admise, grant/génération périmés : refus distinct d’une panne réseau ; aucun
   forward/invoke, effet métier, nouveau Pod ou demande de scaling associée.
   Tester à zéro et à chaud ; l’autorisation de l’admin n’est pas celle de tous
   les callers.
3. **Artefact commun, installations distinctes.** Même release pour deux
   projets/bindings : Services, routes, références de secrets et identités
   distincts. Démontrer refus d’accès croisé avec plugin/canary disponible et
   appel permis de contrôle. Tester aussi deux bindings d’un même projet.
4. **Réveil concurrent depuis zéro.** Appels simultanés autorisés : un seul
   objet logique courant binding/génération, replicas dans les budgets ;
   traces Activator/KPA/Revision/Pod UID et réponse du workload Ready. Vérifier
   l’effet métier, pas seulement HTTP 200. L’idempotence de projection ne
   prouve ni déduplication des invokes ni exactly-once.
5. **Retour à zéro puis nouvel appel.** Absence de trafic pendant la fenêtre
   approuvée, disparition des Pods plugin, route et état métier conservés ;
   nouvel invoke et nouvelle identité d’instance attestée. L’état externe
   requis subsiste ; aucune dépendance à l’ancien filesystem/mémoire.
6. **Anti-contournement et identité.** Chemins froid/chaud, ingress/Activator,
   queue-proxy/user-container, Services/Pods, anciennes Revisions/routes taguées,
   headers forgés, trafic d’un autre binding et egress plugin → Lazaret/système.
   Autorisé fonctionnel en contrôle positif ; refus prouvé au bon hop. Ne pas
   déduire l’isolation d’un DNS absent ou d’un service indisponible.
7. **Révocation et changement de génération.** À chaud, à zéro, pendant
   buffering et en exécution : respect exact de B4, pas de nouvelles
   autorisations ni résurrection par événement/projection/session/Revision
   ancienne. Distinguer effet déjà engagé et opération encore révocable.
   Revocation d’un binding ≠ révocation de la release pour les autres.
8. **Défaillances et bornes.** Image pull, absence de capacité, readiness,
   ingress/autorité/projection indisponibles, saturation et deadline épuisée :
   issue bornée, observable, fail-closed ; pas d’attente infinie ni retry
   silencieux d’opération ambiguë. Rejouer une requête peut dupliquer son effet
   selon le contrat ; `operation_id` seul ne prouve pas le contraire.
9. **Propriété, retrait et migration.** Objet de même nom mais UID tiers,
   double writer, apply retardé, retrait d’un binding partageant la release,
   migration/rollback : stop sur propriété inconnue, préconditions UID,
   aucun effacement de données/objets partagés ni retour au routage digest.
   Suspension de Flux n’est ni arrêt d’accès ni scale-to-zero.

Gates existantes à préserver (cibles exactes à revalider au HEAD d’exécution) :
Manifesto `apparatus_p2_t7_gate`, `apparatus_p3_t2_gate`,
`apparatus_p4_t1_absence` ; Lazaret `apparatus_p3_t7_invoke`,
`apparatus_p3_t9_invoke_prefix`, `apparatus_p3_t11b_session_mtls` ; P4
`apparatus_p4_t12_gate_regression`, admission T7–T9 et contrats P0.
Étendre `apparatus_p4_t10_platform_workload`, `apparatus_p4_t11_runtime` et
`apparatus_m6_e2e_chain` selon le contrat ratifié, sans effacer leurs preuves
historiques ni affaiblir les gates d’absence Kubernetes dans Manifesto.

## 7. Sécurité d’exécution et continuité

- Lire le ledger `.cursor/handoffs/20261019-apparatus-knative-preparation.md`,
  puis créer/mettre à jour celui de l’exécution : base Git, owners/sessions,
  décisions, preuves, tentatives, reste. Réutiliser les découvertes vérifiées ;
  reprendre le même membre pour le même lot, pas une nouvelle exploration.
- Avant de déléguer : profils/configuration de mission confirmés, contrats,
  périmètres, dépendances, invariants et critères de validation par lot.
  La racine est le lead ; aucun coordinateur intermédiaire ni commit/stash/reset
  automatique pour contourner un arbre sale.
- Un seul Cargo à la fois, nativement Windows. Build/check/unit Windows
  pendant l’implémentation ; IT avec fixtures testcontainers existantes, pas
  service applicatif compilé/exécuté dans Docker. ADR 0200–0202 : IT réelles,
  mocks limités aux collaborateurs HTTP sortants, suites opt-in préservées.
- Pas de réveil runtime pour travail documentaire/statique. Avant tout run
  réel, charger `aiforall-test-evidence` et `local-runtime-lifecycle`, inventorier
  l’état avant, les baux et les mutations transitives du harness. Aucun Kind
  avant fin des corrections et phase IT finale. E2E : `kind-aiforall-local` ;
  `kind-aiforall-local-full` seulement sur accord explicite propre au scénario.
  Aucun autre cluster, environnement distant ou production.
- Images Linux pour Kind : build de livraison Docker autorisé dans la campagne,
  jamais montage du target Windows. Enregistrer à la création tous les IDs
  Docker dans `.tmp/docker-resources-<YYYYMMDD>.log` et les UID Kubernetes
  observés/remplacés ; noms et digests ne prouvent pas la propriété.
- Fin de batch IT, succès ou échec : zéro conteneur possédé running, stop
  gracieux par IDs tracés et vérification avant/après selon le canon lifecycle
  du dépôt. Retrait des fixtures uniquement selon ses règles ; aucun prune,
  reset, `down -v`, adoption/force ni nettoyage global implicite. Ne pas toucher
  aux ressources préexistantes/inconnues ou à un workload sous bail.
- Après deux tentatives substantielles sans progrès vérifié sur un même
  blocage, transfert ciblé `emergency-thinker` conformément au harness ; conserver
  hypothèses, essais et faits, sans boucle d’essais ni nouveau GO implicite.

## 8. Hors mission et clôture

Non décidés/non livrés ici : sidecar d’autorisation ou Envoy `ext_authz`, nouvelle
API publique non ratifiée, service de contrôle supplémentaire, charts publisher
privilégiés, partage explicite inter-bindings, exécution non HTTP, Factory/host
P5/P6, cloud bootstrap/ratification 0600, autoscaler de nœuds, scaling prédictif,
coût zéro global et cycle complet de désinstallation/purge.

Le choix éventuel d’un sidecar reste exploratoire : s’il est retenu plus tard,
il ne peut pas, seul dans un Pod endormi, empêcher son réveil non autorisé.
Tout changement de frontière de confiance doit amender l’architecture avant code.

**Clôture ciblée seulement :** preuves du runtime HTTP décrit par 0012 et, si
les critères supplémentaires sont satisfaits et revus, proposition de fermeture
de la Réalité de 0009. Aucune clôture générale pré-prod, de 0010, des `APP-xx`
ou des dettes TCB de 0008. Un rapport partiel reste partiel ; une compatibilité
INCONCLUSIVE ne devient pas un succès par l’Accept de l’ADR.

**État de cette préparation : documentation uniquement ; aucune implémentation,
compilation, IT, installation Serving ou campagne runtime exécutée.**
