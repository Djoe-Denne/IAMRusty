# Tuple NAT unique et rejet ingress Envoy — contrat architecte v2.1

2026-10-29 — auteur architecte/pool-4 ; **Accepted / Partial** : préflight réel réussi ; capture ciblée implémentée,51tests fake et revue sécurité indépendante bornée réussis. Binding et rejet réels restent non qualifiés.
Le lead a lu intégralement les63lignes de la v2 Proposed, comparé la v1 et
ratifie explicitement la v2 sous délégation AVANT code. La v1, non ratifiée,
n'est pas autorisée. Aucun changement ADR, cible, objectif ou policy d'essai.
Complément de la [lecture Admin](local-full-envoy-admin-readonly.md).
Le contrat ingress conditionnel de l'architecte reste Proposed, non ratifié.

## Clarification de décodage v2.1 ratifiée avant code

Le lead a lu intégralement le delta auteur (lignes33–50 de la v2.1 Proposed)
et le ratifie explicitement : aucune nouvelle commande, autorité ou dépendance.
Avec builder fixe conntrack1.4.7/-w0 attesté, exit0 et format compatible :
requested_zone=0. Un token canonique unique zone=0 donne observed_zone=0 et
zone_evidence=OBSERVED_AND_SOURCE_FILTERED. Sans token : observed_zone=MISSING
et zone_evidence=SOURCE_FILTERED, JAMAIS zéro observé synthétisé.

Zone explicite non0/noncanonique/dupliquée, zone-orig/zone-reply même0, format
ou provenance de filtre/version insuffisants : STOP/INCONCLUSIVE. Aucun filtre
retiré, alias directionnel, autre zone ou dump supplémentaire. Les autres champs
obligatoires manquants restent refusés. Les dépendances installées ne sont pas
qualifiées par ce raisonnement :1.0.9/6.6 sont des références ABI, pas des versions
prétendues du laboratoire.

Sources auteur vérifiées : conntrack-tools1.4.7 man217–218 ; conntrack.c254–257,
3063–3069,1669–1679,2643–2647 établissent la comparaison de zone avant impression.
Libnetfilter_conntrack1.0.9 compare.c17–34,379–386,461–462 et api.c512–516
expliquent la comparaison du défaut non imprimé ; snprintf_default.c486–489
n'imprime la zone que si ATTR_ZONEset,432–435/455–458 distinguent les directions.
Linuxv6.6 nf_conntrack_netlink.c153–160 omet le défaut défini0 dans zones_common.h7.
Cette omission n'autorise pas à fabriquer une valeur observée.

Oracles avant run : test_envoy_tuple_zone_missing_preserves_source_filtered_provenance
et test_envoy_tuple_zone_conflicts_and_unattested_filter_fail_closed. Format
compatible sanszone=>MISSING/provenancefiltrée ; token0 explicite positif ;
non0/dupliqué/directionnel/filtre ou version nonattestés=>STOP. Aucun live auteur.

## Décision et limites

Retenir un tuple conntrack filtré sur le nœud d'entrée et une seule chaîne
Calico du Pod, pas de dump NAT ou graphe KUBE/Calico. Aucune capacité IG utilisable
attestée ; pas de recherche élargie, gadget, MCP ambiant ou installation.
Outils, modules, backend, relation DockerDesktop/Windows et absence de concurrence
restent à établir. Échec ou ambiguïté : STOP/INCONCLUSIVE, pas de fallback ou patch.

Le binding kernel peut établir la source traduite, pas à lui seul l'attribution
d'un rejet à cette connexion. Le verdict reste séparé en NAT_BINDING et
POLICY_DROP_CORRELATED ; aucun PASS causal automatique. Les deux sont nécessaires
avant ratification/correction ingress, avec exclusion étayée des alternatives.

## Autorité et préflight fermés

Réutiliser les gardes endpoint/CP/roster/config/hash/ACL/UID et possession
namespace/Deployment/ReplicaSet/Pod/incarnation. CPID intégral égal au bail ;
workerID dérivé de NodeUID/NodeName d'Envoy et du roster, jamais deviné. Envoy
full uniquement, PodIPv4 unique, containerID/restartCount/imageID et ServiceUID
NodePort30080→10000/Cluster stables avant/après. Invalides : zéro commande de
diagnostic. Refus natif : arrêt, jamais autre runner.

Préflight via helper seulement, argv internes fixes sans shell :
- docker exec <CPID> conntrack --version
- docker exec <workerID> iptables --version
- docker exec <CPID> test -d /sys/module/nf_conntrack
- docker exec <CPID> test -d /sys/module/nf_conntrack_netlink

Modules déjà résidents/builtin exigés avant netlink. Une absence n'est pas une
preuve globale, mais interdit un essai provoquant le chargement. Pas de modprobe,
sysctl, autoload intentionnel, test via shell, événement ou trace.
Sémantique vérifiée conntrack1.4.7 ; autre version/format exige recoupement
spécifique avant utilisation. Backend iptables réellement accessible reconnu,
pas déduit de CalicoReady. Binaire/backend/module absent : INCONCLUSIVE, sans
installation, autre binaire, nft ou libkernel substitué.

## Lectures minimales

Aucun argv, ID, hôte, table, chaîne, filtre ou callback libre au caller. Les IDs
sont dérivés/attestés par le garde. Aucun privilège/user/env ajouté, pipe, shell,
stdin interactif, copie de fichier ou exécution brute offerte au caller.

1. Binding CP filtré exclusivement Service/endpoint/zone0 :
   docker exec <CPID> conntrack -L -f ipv4 -p tcp
   --orig-dst <CPIPv4 attestée> --orig-port-dst 30080
   --reply-src <EnvoyPodIPv4 attestée> --reply-port-src 10000 -w 0 -o extended,id
   Borne16KiB,8entrées max. Pas de table entière, zone/proto libre, zéro, création,
   modification, suppression, flush, expect, événement ou timestamp configuré.
   Aucun défiltrage si rien n'est trouvé. Le filtre de sortie ne promet pas un
   parcours interne netlink limité à une entrée. Décodeur étroit original/reply
   IPv4/ports, TCP/état/id/zone0 ; doublons/format inconnu/manque : INCONCLUSIVE.
2. Liaison worker : docker exec <workerID> ip -j -4 route show exact <PodIPv4>/32.
   Une route/dev locale unique, interface cali normale<=15caractères. Adresse
   exclusivement issue du Pod attesté. Aucun autre sous-verbe ip/adresse libre.
   Chaîne dérivée seulement par EndpointChainName Calico3.29.7 non tronquée/non
   hachée pour cette longueur et backend confirmé ; sinon STOP, pas graphe.
3. Une chaîne ingress : docker exec <workerID> iptables -t filter -L <chaîne>
   -n -v -x --line-numbers. Borne64KiB/128règles. Direction cali-tw VERS workload,
   jamais cali-fw FROM/egress. Seuls DROP/REJECT directs du template « aucune
   policy passée » sont candidats ; INVALID/encapsulation/autre rejet ne valent
   pas refus de source. Commentaire seul insuffisant. Template/marks/format
   inconnus : AMBIGUOUS, sans interpréteur général ni suivi de graphes/ipsets.

Commandes5s, snapshot30s, sans retry. Version/route16KiB, stderr16KiB.
Capture réellement bornée avant parse, UTF8strict, stdout/stderr privés ; aucune
capture illimitée suivie d'un test de longueur. Childhandle possédé arrêté/joint
et pipes fermés sur échec, aucun processus étranger. Pas de sortie brute des
tuples historiques, règles, commentaires, environnement ou Secrets.
Retour fermé : identités/liens, empreintes structurelles hors compteurs,
compteurs uint64 et seul nouveau tuple retenu IP/ports/id validés. MISSING jamais0.

## Séquence et interprétation

Parent : baseline fermée, puis UNE connexion TCP fraîche vers l'entrée exacte
http://127.0.0.1:18080/iam/api/auth/gitlab/login, sans keepalive/proxy/redirect/credential ajouté,
délai15s ; post immédiat. Aucun navigateur OAuth/test parallèle ciblant ce
Service ; concurrents/probes documentés, pas supposés absents. Helper sans URL
ou callback libre. Pas de retry/batch/connexion de substitution.

Un tuple nouveau unique avant/après, id et original/reply cohérents, identités
et fenêtre bornées. Zéro/multiples/ancien/réutilisé/expiré/format ambigu ou drift
backend/règles/route/UID : INCONCLUSIVE. IDkernel n'est pas horodatage attesté.
Ne pas activer timestamp/accounting/sysctl. Le socket Windows ne donne pas les
champs original.src/sport CP : DockerDesktop peut les traduire. Si la différence
temporelle unique ne lie pas la ligne au test, STOP, jamais retrait de filtre.

Original S:s→CP:30080, reply Pod:10000→T:t : T:t est le couple source traduit
enregistré vers Pod. Source1.4.7 : SNAT=ATTR_REPL_IPV4_DST et DNAT=ATTR_REPL_IPV4_SRC.
UNREPLIED n'invalide pas ce binding mais reply n'est pas une réponse observée :
ni réception Envoy, ni verdict worker prouvés. Si T diffère de CPInternalIPv4,
signaler le fait sans changer automatiquement l'IPblock du contrat ingress.

Rejet : même Pod/interface/chaîne/empreinte/backend, compteurs monotones et hausse
du deny pertinent, concurrence exclue de façon étayée. Pas d'égalité1 imposée :
NAT compte surtout la nouvelle connexion, filter peut compter les retransmissions.
Une chaîne endpoint partagée n'identifie pas magiquement le tuple. Autre flux
possible, template non discriminant ou liaison incertaine : DROP non qualifié.
Absence de hausse ne prouve ni allow ni autre rejet. La fenêtre passive aide,
sans garantir absence de concurrence future.

Si les deux faits ne lèvent pas les préconditions ingress : vraie frontière =
observation par-flow du rejet/liaison de requête, indisponible dans les moyens
actuellement attestés. Aucune permission implicite de trace, conntrackevents,
iptables-rule de comptage ou policy d'essai ; pas de capacité plus grosse pour
fabriquer un vert. Timeout/UNREPLIED/zéroAdmin/règle ne suffisent pas seuls.

## Allowlist et validation avant run

Deux fichiers : ops/deploy/local_full_target.py et ops/tests/test_local_full_target.py.
Pas de CLI générique, plateforme/harness, image, SDK/app, Netpol/RBAC, permissions
ou config globale. Anciennes opérations inchangées. Première tranche de travail
choisie par le lead : préflight fermé seulement, avant décodeurs de tuples/rejet.
Pas de lecture conntrack ni règle worker tant que cette tranche n'est pas qualifiée
et que l'outillage réel n'est pas disponible. Ce séquençage ne ratifie aucun PASS.

Oracles nommés avant code/run :
- test_envoy_tuple_preflight_rejects_missing_tools_modules_and_foreign_nodes_before_dump
- test_envoy_tuple_queries_are_closed_and_original_reply_nat_mapping_is_exact
- test_envoy_tuple_requires_unique_new_candidate_and_stable_identity
- test_envoy_tuple_drop_chain_is_to_workload_and_ambiguous_deltas_do_not_pass
- test_envoy_tuple_bounds_and_failures_never_echo_private_output

Préflight : outils/modules/permission absents=>STOP sans netlink/install/fallback,
CP/worker dérivés. Requêtes : argv fermés et directions NAT, UNREPLIED distinct
d'une réponse, originalDocker différent du socketWindows positif. Comparaison :
zéro/multiples/anciens/drift refusés, positif unique et limites8/16KiB. Chaîne :
tw liée vs fw/INVALID/concurrence/reset/missing. Bornes :16/64KiB, délai/UTF8/
sentinelles/childclosed, aucune sortie privée. Suite unittest existante, sans
nouveau runtime de tests ; code et revue nécessaires avant accès réel.

## Sources architecte et continuité

- conntrack-tools1.4.7 source et man Debian versionnés :
  https://sources.debian.org/data/main/c/conntrack-tools/1%3A1.4.7-1/src/conntrack.c
  https://sources.debian.org/data/main/c/conntrack-tools/1%3A1.4.7-1/conntrack.8
  Upstream netfilter ciblé renvoyait une page antibot, non utilisée comme code.
- Calico3.29.7 felix/rules/endpoints.go et rule_defs.go : EndpointChainName,
  préfixes tw/fw, template ingress/deny. Kube-proxy1.32.2 reste source d'attente,
  pas interprété comme graphe ou preuve effective.

Original auteur63lignes conservé dans le worktree pool-4, même nom de fichier ;
v2 finale native msg_18716b9fa001NGymVWTshpp9dz/session ses_e7c23de99ffe01g8pHD4492lnP.
Ratification conserve les garanties et limites de cette v2 ; aucun accès live,
code, test, Cargo, SecretGET ou module exécuté par l'architecte.
