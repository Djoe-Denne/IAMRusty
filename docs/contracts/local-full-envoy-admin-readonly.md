# Capacité spécialisée de lecture Envoy Admin du laboratoire full

Version 1, 2026-10-29. Auteur : architecte/pool-4. **Accepted / Partial** : code intégré, tests fake et revue sécurité bornée réussis ; comportement et cycle de vie réels encore non qualifiés.
Base application636b438, SDK02e5. Le lead a lu les51lignes du contrat original et
ratifie explicitement ses exigences et son risque local ci-dessous, sous délégation
utilisateur et AVANT code. Complément opérationnel de l'alignement0412, pas de
nouvelle ADR ni de modification de Statut/Réalité.

## Décision et limite déterminante

Créer une capacité spécialisée de snapshot Admin réutilisant ancrage, propriété
et cycle de vie du helper existant, sans élargir oauth_forward ou diagnostic_logs.
Aucune solution existante conforme : oauth_forward est figé sur le simulateur,
les logs ne fournissent pas les compteurs et curl manque dans l'image exécutée.

**API GET fermée ne signifie pas tunnel TCP en lecture seule.** Envoy1.31.2 Admin
est non authentifié et comporte des POST destructifs. Le tunnel127.0.0.1 expose
temporairement l'interface complète aux autres processus locaux. Le lead ratifie
EXPLICITEMENT cette exposition éphémère sur le poste de campagne de confiance,
dans ce laboratoire uniquement. Ce n'est pas une défense contre un processus ou
navigateur hostile local, ni contre CSRF. Pas de tunnel pendant une navigation
OAuth active, aucune adresse donnée au navigateur. Pas de qualification production,
d'hôte partagé non fiable, d'accès LAN, proxy, firewall ou permission ajouté.

Extension opérationnelle bornée, pas de nouvelle topologie ou racine de confiance
produit. Refusés : kubectl brut/get--raw/Dockerexec/shellTCP, installationcurl,
rebuildEnvoy, retarget du forwardOAuth, forward générique ou nouveau proxy.

## Identité et cycle de vie

1. API sémantique envoy_admin_snapshot retournant seulement les projections
   fermées : pas d'URL, path, query, callbackHTTP, adresse, port ou handle libre.
2. Cible : kind-aiforall-local-full/aiforall-local-full, namespace aiforall-local-full,
   Pod exact de Deploymentenvoy-mesh, containerenvoy, port distant9901. Le parent
   atteste namespaceUID, PodUID, ReplicaSetUID→DeploymentUID possédé et incarnation
   containerID/restartCount/imageID. Noms, labels ou image seuls ne suffisent pas.
   Refuser containers/init/ephemeral inattendus : le tunnel vise le réseauPod,
   pas un container via -c.
3. Entrée invalide : zéro probe. Ancrage endpoint/CP/roster/hash/ACL avantUIDAPI,
   puis kube-systemUID, namespaceUID, chaîne possédée et incarnation par projections
   sans env/args/volumes/Secret. Revalider avant lancement, chaque lecture et après
   capture avant retour. Changement/absence/indécidable : discard, close et
   INCONCLUSIVE, sans adoption ou retry. Une course résiduelle par nom reste possible :
   pas de précondition PodUID atomique, ni garantie d'absence absolue de course.
4. Enfantkubectl privé avec prefix kubeconfig/context/env fixe ; pod/nomexact,
   namespace exact, bind127.0.0.1, mapping:9901 alloué par kubectl. Pas de Service,
   port fixe ou check-then-bind. Listener, Service, TLS et entrée18080 inchangés.
5. Marqueur privé exclusif.envoy-admin, distinct d'OAuth. Stale : STOP, jamais
   adoption ou kill par PID. PID informatif seulement, propriété par handle créé ici.
   StdinDEVNULL, stdout/stderr bornés privés, parsing strict de readiness sans écho.
   Adresse ou mapping ambigu/étranger : refus, aucune adresse rendue au caller.
6. Readiness5s, GET5s, tunnel total30s ; au plus un server_info et un jeu de
   compteurs par opération, pas de polling/retry. Snapshots distincts pour un delta,
   avec même incarnation ; aucun tunnel maintenu pendant la campagne navigateur.
7. Finally ferme réponses/sockets ; arrêt gracieux du seul childhandle, join15s.
   Marqueur retiré seulement après fin attestée de cet enfant ou non-lancement
   prouvé. Cleanup indécidable : marqueur conservé et erreur, pas PASS ou keepalive.

## Lectures et projections fermées

Client stdlib sans proxy, cookies ou credentials ambiants. Destination interne
127.0.0.1 et port de son enfant, jamais un input caller. GET seulement, sans body,
Authorization, cookie, redirect ou compression. Autre statut que200 : erreur
constante sans lecture/écho du corps. Lecture incrémentale avec vraie borne et
budget wall-clock, pas Content-Length seul.

- /server_info, au plus16384B : état enum, égalité version/image attendue,
  restart_epoch et uptimes numériques valides. Aucun JSON brut, option ou chemin.
- /stats?format=json&filter=regexinterne, au plus65536B : regex ancrée et échappée
  générée depuis clusters{ext_authz,iam,hive,telegraph,manifesto} et suffixes
  {upstream_cx_total,upstream_cx_connect_fail,upstream_cx_connect_timeout,
  upstream_rq_total,upstream_rq_5xx,ssl.handshake,ssl.fail_verify_san,
  ssl.fail_verify_error}. Sous-ensembles enum fermés possibles, pas filtre libre.

Refuser encodage/JSON malformé, doublons, noms inattendus, entiers négatifs ou
non entiers. **MISSING reste MISSING/INCONCLUSIVE, jamais zéro inventé.** Retour
limité aux noms constants, entiers et états corrélés à l'incarnation. Pas de dump,
indexation ou git d'Admin brut, usedonly/config_dump/certs/runtime/clusters brut,
histogram/Prometheus, logs/Secret/JWT ou endpoint mutation. Aucun POST/PUT/DELETE.

Deltas : mêmes Pod/container/incarnation/restart_epoch et compteurs monotones.
5xx/connectfail seul ne prouve pas SAN. Préserver le vrai oracle mêmeCA/bonnom
contre mêmeCA/mauvaisnom et hausse SAN distincte d'erreurCA. Ce contrat n'autorise
pas lui-même une mutation de certificat/backend, ni un changement d'entrée0412.

## Périmètre et vérification future

Allowlist : ops/deploy/local_full_target.py et ops/tests/test_local_full_target.py,
sans refactor général ou modification oauth_forward/diagnostic_logs. Adaptation
de l'oracle live attribuée séparément avec chemin exact par le lead. Aucun code
app/SDK/Cargo/image/déploiement/RBAC/SecretGET/pluginEnsemble/configglobale ou
permissionnative modifié. Un refus natif reste bloquant.

Tests inscrits avant code/run :
- test_envoy_admin_rejects_foreign_target_and_free_requests_before_probe
- test_envoy_admin_requires_owned_chain_and_stable_container_incarnation
- test_envoy_admin_exact_child_mapping_and_closed_gets
- test_envoy_admin_limits_and_missing_stats_are_not_zero
- test_envoy_admin_stale_marker_and_cleanup_use_only_owned_handle

Oracles : invalides avant probe ; chaîne/incarnation positive puis mismatch et
changement avec discard ; prefix/env/mapping/GET/projections exacts ; bornes/chunks/
slow/erreur/JSONdoublons/sentinelles, zéro réel distinct de missing ; succès,
exception/startupfail/timeout, seul handle arrêté/joint, stale intact et marqueur
retenu si cleanup non attesté. Tests fake, sans runtime.

Suite : python -B -m unittest discover -s ops/tests -p test_local_full_target.py.
Préserver les tests existants. Les mocks ne qualifient pas le réseau ou SAN réel.

## Sources architecte

Helper kubectl/diagnostic_logs/oauth_forward ; Adminloopback9901 et image1.31.2
danskind-mesh/envoy-mesh.yaml ; test_envoy_upstream_server_identity.py.
[Admin1.31.2](https://www.envoyproxy.io/docs/envoy/v1.31.2/operations/admin) et
[TLSstats1.31.2](https://www.envoyproxy.io/docs/envoy/v1.31.2/configuration/listeners/stats)
consultés par architecte2026-10-29. Noms upstream exacts à constater dans les
snapshots réels, pas à déduire de la documentation listeners. Aucun runtime
exécuté par l'auteur ; original Proposed préservé dans son worktree.
