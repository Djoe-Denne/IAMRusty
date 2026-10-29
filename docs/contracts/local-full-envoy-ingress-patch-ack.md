# Amendement — postimage ACK du patch ingress possédé

2026-10-29 — auteur architecte/pool-4 ; **Accepted / Unimplemented**.
Le lead a lu intégralement la proposition finale de82lignes et ratifie explicitement
cet amendement AVANT toute extension du garde ou implémentation ACK/provenance,
sous la délégation utilisateur de cette mission. Cette ratification approuve la
forme fermée, **aucun hash de helpermutation futur** : sa paire exacte et les sept
empreintes devront être approuvées après code figé, tests et revue.
Original Proposed conservé au worktree auteur, même nom de document.

Complément du [contrat ingress v2](local-full-envoy-nodeport-ingress.md), base
canonique95b912ee01859f9e1b62ed94ef072802c1cb5343 et briefing
`.cursor/review-briefings/20261029T1233Z-security-envoy-ipip-ingress-ff12007.md` S-1.
Aucun changement d'ADR, cible ou autre capacité. Cet amendement prime seulement
les mentions « relecture/mismatch ⇒ rollback conditionnel » et « RVcourante »
du §Patch du contrat ingress pour la propriété du même objet : une révision
concurrente ne peut jamais être adoptée ; le rollback doit tester RVACK.
Source/NAT/causalité/L3 et ADR Accepted restent inchangés.

## Motif et décision

Le runner ignore la réponse PATCH puis fige la resourceVersion du premier GET.
Un concurrent peut retirer/réintroduire la même règle sur le même UID entre ces
opérations : spec identique mais nouvelle RV, adoptée à tort comme possédée.
C'est une course sur le même objet, pas la non-atomicité multiobjets acceptée.
S-1 reste bloquant jusqu'au correctif et à ses oracles ; aucune dispense.

**Seule la réponse effective de notre mutation attribue la postimage possédée.**
Obtenir JSON par le transport kubectl/garde existant, figer UID/RV/spec depuis
cet ACK, puis comparer les relectures à CET ACK. Aucun GET ne peut créer ou
renouveler cette autorité. Préserver tests UID/RV/ancien spec et autres garanties.

## Extension minimale de grammaire, avant toute probe

Seule forme nouvelle :
`patch networkpolicy/allow-envoy -n aiforall-local-full --type=json -p <JSONPatch-inline> -o json`

- UN target exact `networkpolicy/allow-envoy`, namespace explicite full, typeJSON,
  payload inline tableau JSONPatch non vide, sortie json seulement. Le namespace
  long déjà supporté peut rester admis s'il est unique. Pas nouvel alias. Validation
  indépendante de l'ordre des options admises.
- `-o json` unique, standalone ; pas -ojson/-o=json, yaml/name/jsonpath, --output,
  cible supplémentaire, namespace implicite/étranger, duplicats même égaux ou `--`.
  Aucun patch-file/-f, local/dry-run/record/subresource, autre type de patch ou
  méthode de remplacement dans ce modeACK.
- Secret/se/secrets, autre ressource/nom/namespace et target/auth override
  (context/kubeconfig/server/user/token/impersonation/TLS...) refusés AVANT
  Docker/UID/API. Ne pas généraliser le validateur output aux formats de patch.
  Le garde ferme shape/target ; le runner vérifie opérations, tests et ownership,
  sans recopier le jugement causal dans le parseur.
- Anciennes opérations sans ce mode de sortie inchangées. Pas nouvelle lecture
  générique de mutations ou de Secrets, contournement natif ou changement des
  contrôles d'ancrage/endpoint/CP/config/UID/prefix après validation.

## ACK, continuité et rollback

1. Journal privé pré-mutation, preuve/source/UID/RV/spec attendus. UNE invocation
   JSONPatch testé avec `-o json`, sans apply/curl/watch/autre transport. Exit0 ou
   texte « patched » ne constitue pas un ACK exploitable.
2. Parser l'objet NetworkPolicy de l'API attendue : name/namespace exacts, UID
   égal à la préimage, RV chaîne opaque non vide, spec postimage attendu, pas de
   deletionTimestamp. Pour un ajout effectif, RVACK diffère de la pré-RV ; aucune
   comparaison numérique/lexicographique d'ordre des RV.
3. AVANT tout GET ultérieur, figer par copies/valeurs immuables UIDACK/RVACK/specACK
   liés au journal/opération. Reçu = copie, pas alias. Conserver ces champs dans
   le reçu/journal privé sans dump d'objet/annotations dans stdout ou rapport.
4. GET/revalidation/finish retrouvent UIDACK ET RVACK ET specACK, name/namespace
   et absence de suppression. Spec égal mais RV différente = concurrence : STOP
   sans adoption, actualisation de RVACK ou retrait incertain. Aucun retry ou
   récupération d'ownership par un autre GET.
5. ACK absent/perdu/tronqué/malformé, transport en erreur après mutation possible,
   UID/spec/RV incohérents : STOP, journal d'état incertain conservé, aucun retrait
   automatique. Un GET diagnostique ne fabrique jamais une propriété ou un droit
   de réessai. Échec de réception ne prouve pas absence de mutation.
6. Rollback seulement d'un ajout ACKé encore inchangé : GET égal à UIDACK/RVACK/
   specACK, patch atomique testant LA RVACK FIGÉE, UID/spec/entrée, retrait de cette
   seule entrée. Aucune RV fraîche substituée, restauration globale, adoption d'un
   no-op ou retrait d'un objet remplacé. Concurrence avant GET ou entre GET et
   mutation => refus fermé, aucune mutation étrangère.
7. Le rollback reçoit aussi son ACK JSON par cette forme fermée. Valider UID,
   spec restauré et nouvelle RVACKrollback ; postGET doit égaler CET ACK. Perte,
   mismatch ou concurrence ensuite => STOP et journal précis, aucune répétition,
   remise ou nouvelle adoption/retrait. Distinguer ACK reçu et état courant.

Limite : fermeture de la course connue, pas livraison garantie de l'ACK, absence
de futurs écrivains ou atomicité Node/Pod/Service/policy. Perte d'ACK peut laisser
un changement réel non automatiquement retirable ; revue parent non destructive,
jamais élargissement des permissions/réseau pour forcer le cleanup.

## Provenance capture→mutation — qualification fermée

**Allowlist versionnée de codes qualifiés dans le runner**, pas booléen ou fichier
runtime à hashes libres. Distinguer helper producteur de capture et helper courant
dont la grammaire change, sans abandonner leur vérification.

- Sept projections/source/verdict restent byte-immutables : observation, summary,
  baseline, passive-baseline, admin-before, admin-after, causal-review. Pas rehash
  de summary, changement de verdict, faux ancien producteur ou ancien helper
  exécuté comme autorité. Conserver le manifest SHA complet déjà calculé.
- Entrée unique pour cette observation : capture_ops_head complet,
  capture_helper_RAW_SHA256, observation_SHA256 et manifest exact des sept fichiers
  vers mutation_helper_RAW_SHA256 qualifié + référence d'audit. Aucun préfixe,
  wildcard, regex, plage, CLI/env, hash JSON libre ou défaut permissif.
- AVANT édition, snapshot RAW ancien dont le SHA égale le producteur de summary.
  Git/reconstruction recevable seulement avec octets retrouvant EXACTEMENT ce RAW,
  pas normalisation du hash de capture. Après code, octets RAW du helpermutation
  livré figés et hashés. Parser les textes pour audit, jamais les importer comme
  autorités alternatives.
- AST entier hors SEUL CORPS de parse_kubectl_operation identique, positions et
  commentaires ignorés ; signature/décorateurs/retours inchangés. Imports,
  constantes, classes, ancrage, parsers/collecteurs tuple, Admin, lifecycle et
  dépendances transitives du module inchangés. Audit du corps/diff : seule
  exceptionACK ciblée, pas import dynamique/globalwrite/monkeypatch/fallback ou
  comportement ancien changé. AST seul ne prouve pas sûreté du corps : tests et
  revue S-1 obligatoires. Toute sortie d'enveloppe => STOP, pas exclusion AST élargie.
- APRÈS tests/revue du code figé, le lead approuve la PAIRE EXACTE et le manifest
  avant activation de l'entrée. Forme ratifiée n'approuve aucun hash futur à blanc.
  Valeurs/audit manquants => aucun chemin de compatibilité actif ; aucune mise à
  jour automatique et chaque future paire doit être qualifiée.
- Pour l'observation historique connue : vérifier EN PRIORITÉ entrée exacte,
  sept hashes, producteur/ops_head et hash RAW du helpermutation réellement importé.
  Mismatch => STOP AVANT branche d'égalité : summary réécrit au nouveau hash ne
  transforme pas cette capture en capture nouvelle. Hors observation historique,
  égalité producteur/autorité garde ses contrôles stricts ; aucune compat inconnue.
- Garder runtime.LocalClusterTargetAnchor is target_authority.LocalClusterTargetAnchor,
  chemin/source réellement importés et gardes. Invocation Python fraîche après
  gel : pas module/instance d'avant édition ou fallback sur snapshot. Figer dans
  le reçu producteur, mutation et entrée qualifiée ; vérifier autorité inchangée
  prévalidation/ACK/finish/rollback. Une autre version qualifiée ne renouvelle pas
  l'autorité d'une opération ouverte.

Preuves supplémentaires minimales : sourceRAW ancien et reçu d'audit/qualification
parent privés/scellés, création exclusive à côté de la campagne, sans écraser les
sept projections. Reçu lie deux RAW_SHA, manifestseptSHA, portée AST/diff,
tests/revue/décision exacte ; traçabilité, PAS auto-approbation runtime de hashes.
Source nouveau livré/figé peut être référencé plutôt que dupliqué s'il est conservé.
Pas Secret/probe/recapture, nouveau service, schema produit, crypto ou bibliothèque.
Aucune recapture NAT par principe pour le seul parseur qualifié ; changement réel
du collecteur/Admin/ancrage => réexaminer le besoin précis, jamais compat automatique.

## Allowlist et oracles

Quatre fichiers SEULEMENT pour ce correctif :
- ops/deploy/deploy-local-full.py : ACK/rollback, copies, comparaison, journaux,
  allowlist exacte et manifest/source réellement importée.
- ops/tests/test_local_full_envoy_ingress.py : ACK COPIÉ au point de mutation,
  pas référence vivante/texte « patched » ; course/incertitude/provenance.
- ops/deploy/local_full_target.py : seule exception de sortieJSON de patch,
  validation avant probe ; aucune autre capacité.
- ops/tests/test_local_full_target.py : positifs/négatifs, autres gardes préservés.

Noms à inscrire avant exécution, tests précédents conservés :
- test_envoy_ingress_does_not_adopt_equal_spec_update_between_ack_and_first_read
- test_envoy_ingress_patch_ack_is_frozen_and_lost_ack_never_grants_rollback
- test_envoy_ingress_rollback_uses_ack_rv_and_does_not_adopt_postrollback_get
- test_envoy_patch_json_ack_grammar_is_exact_before_any_probe
- test_envoy_ingress_qualified_capture_helper_pair_preserves_immutable_evidence
- test_envoy_ingress_unknown_helper_pair_or_rewritten_historical_summary_is_rejected
- test_envoy_ingress_mutation_authority_cannot_change_after_ack
- Gate source : ASThorscorpsparse sur vrai RAW et négatifs collecteur/Admin/
  constante/import/lifecycle refusés, sans nouveau framework ou comparateur prod.

Oracle Admin natif/JSONroundtrip et négatifs UID/incarnation restent un correctif
compatible runner/test, sans relâchement ou nouvelle autorité de rollback.
Suites unittest des deux fichiers ciblés, revue bornée du delta S-1 intégré avant
live ; pas nouveau harness/pipeline, app/SDK/image/Service/TLS/Netpolsource/Cargo.
La rédaction architecte n'a exécuté aucun test/live/API.

## Sources et alternatives

Canon95b912e, runner:383–415,489–529 ; garde:96–180 ; briefing freshS-1.
Source officielle kubectl1.32.2 :
https://github.com/kubernetes/kubernetes/blob/v1.32.2/staging/src/k8s.io/kubectl/pkg/cmd/patch/patch.go#L260-L285
helper.Patch renvoie patchedObj que le printer émet ; --record peut produire un
second patch, d'où exclusion. Rejetés : GET plus rapide/spec seul/retryRV/watch,
apply/curl/kubectl direct, nouvelle lease/verrou distribué. Aucun transport,
ADR ou permission produit nouvelle ; rétablissement d'un invariant de propriété.
