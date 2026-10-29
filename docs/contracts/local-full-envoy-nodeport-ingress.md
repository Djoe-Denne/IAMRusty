# Ingress du tunnel CP mesuré vers Envoy full — contrat architecte v2

2026-10-29 — auteur architecte/pool-4 ; **Accepted / Implemented, qualified laboratory ingress only**.
Le lead a lu intégralement les55lignes de la v2 Proposed et le rapport RO21,
puis ratifie explicitement cette version sous délégation AVANT code/patch.
La propositionv1 CPInternalIP/32, non ratifiée et contredite par la mesure, est
abandonnée. Aucun changement d'ADR Accepted, de cible, permission ou plateforme.
Entrée inchangée :127.0.0.1:18080→CP30080→Envoy10000.

## État vérifié du29octobre — ne change pas la ratification initiale

Implémentation livrée3c2e7ea, après correction du catalogue des trois règles du
profil OAuth existant. Une entrée mesuréeT/32TCP10000 ajoutée avec UID/RV/spec
issus de la réponse du patch ;24autrespolicies inchangées. Témoinréel303 et trois
refus403 sanscookie, vérification en mémoire d'absence de dispatchIAM des seuls
négatifs. Relecture indépendante documentaire de l'ACK et des25policies : PASS
borné ; détails et limites dans le [rapport d'entrée](../../.opencode/missions/team_mvsc2v49_0000_d9lbvu/reports/61-entree-reseau-retablie-refus-authentification-conserves.md).

Le cycle Envoy bon/mauvaisnom signé mêmeCA/bonrestauré a également réussi ; ce
n'est ni une qualification de production, ni la validation du navigateur.
La campagne navigateur a échoué au callbacklogin401 après inscription ; elle
reste ouverte. Les compteurs Admin5xx absents restent MISSING et le delta IAM
propre aux négatifs n'est pas recalculable depuis les seuls snapshotsglobaux
archivés. Ne pas étendre ce résultat à une nouvelle capture/source ou autorité.

## Décision et risque L3 ratifiés

Autoriser UNE règle ingress dans allow-envoy/aiforall-local-full possédée :
sourceT/32, T à la fois reply.dst du binding unique mesuré et champ IPIP du CP
ancré attesté. Destination podSelector envoy-mesh, TCP10000 seulement. Source
dérivée et mesurée au run, jamais IP de configuration ou argument libre sans preuve.
Aucun fallback InternalIP, autre tunnel/nœud, PodCIDR/bridgeCIDR/0/0 ou header.

Le lead ratifie EXPLICITEMENT la confiance L3 du tunnel CP dans ce laboratoire :
elle admet tous les flux présentant cette source relayée, pas seulement un
navigateur127.0.0.1. Ce n'est ni identité utilisateur ni garantie anti-spoof
cryptographique. JWT/authN/authZ, exemptions, routes exactes et TLS/mTLS/SAN restent
inchangés. Aucune qualification production ou permission nouvelle en découle.

Conserver default-deny, peer local-full-client, tous egress/autres policies,
ServiceNodePort30080→10000/TrafficPolicyCluster, labels, destinations et listener.
Pas de schedulingCP, policyLocal, proxy/clientPod substitué au navigateur,
nouveau mapping, suppression/adoption/recréation de policy ou apply global.

## Gates acquis et jugement causal borné du lead

1. Ancrage : bail/CPID/endpoint/kube-systemUID, namespace, EnvoyPod/ownerchain/
   incarnation/worker et Service de la capture ff12007 attestés, AdminLIVE et
   fermé. Avant mutation, revalider les valeurs sémantiques et EndpointSliceReady,
   policyUID/RV/spec/ensemble applicable ; dérive=>STOP, jamais MISSING→0.
2. Liaison RO21 : CPNodeUIDe94277f9-0a57-406f-9479-cb2dbc5cda5b porte
   projectcalico.org/IPv4IPIPTunnelAddr=10.244.230.64, unique sur les3Nodes.
   IPv4Address172.19.0.5/16 concorde avec InternalIP ; VXLAN absent. Worker2
   porte une autre adresseIPIP. La capture a déjà lié CPID intégral du bail à
   NodeName/UID par DockerName+Hostname+IPv4 et API avant/après. Le suffixeproviderID
   n'a PAS prouvé ce lien. La sonde CRD refusée n'est pas contournée ni revendiquée.
3. Provenance : observation fermée SHA256
   0ad1c639efe15aa5a8e183a95a5171e761fba7fa2d3e2c0d841c5720f8781280,
   binding uniqueoriginalversCP30080/replydepuisEnvoy10000 versT, ID575729818.
   SYN_SENT/UNREPLIED n'est pas réponsevue ; zoneobservéeMISSING/filtre0 distincts.
4. Jugement lead : **QUALIFIED_EQUIVALENT_LAB_ONLY**, pas trace par paquet.
   Le décodeur revu identifie la chaîne tw liée au veth et son unique denyterminal
   MarkClear/policyjump inbound, pas INVALID/encapsulation. Référence passive
   22paquets/1320octets stable, puis +9/+540 après UNE requêteexclusiveparent.
   Empreintes/UID et policiesobservées stables, EnvoyLIVEepoch0, IAMsanshausse.
   Aucun autre test/OAuth/clientmission actif visant cette entrée ; les seuls
   travaux concurrents sont RO/documentaires. Les sondes TCP du nœud résident
   sont inchangées et ne faisaient pas croître ce compteur pendant la référence.
   Default-deny egress et destinations déclarées ne créent pas un autre client
   applicatif connu ; le clientPod explicitement admis n'explique pas ce refus.
   La différence temporelle unique et le refus attendu de la source nonclient
   discriminent le hop dans cette campagne contrôlée. Les alternatives pertinentes
   connues sont exclues, pas toute concurrence hostile hypothétique sur l'hôte.

Le compteur reste POD_CHAIN_SHARED/NOT_ESTABLISHED_BY_COUNTER dans les artefacts,
la référence passive ne garantit pas absence de burst futur et son hashNetpol
n'est pas un hash du grapheCalico complet. Ce jugement distinct ne réécrit pas
PENDING_LEAD_CAUSAL_REVIEW du collecteur. Si une alternative pertinente, dérive
ou attribution contradictoire apparaît : INCONCLUSIVE et aucun patch d'essai.

## Continuité de la liaison

T doit rester égal au tunnelIPIP canonique du même CPNodeUID/CPID, unique parmi
le roster du lab, sans attribution contradictoire. Reprendre l'identité stable
de la preuve, pas un Node choisi par nom ou IPseule. En KDD, CalicoNode peut être
une vue des mêmes annotations avec OrchRefk8s : pas une seconde preuve indépendante
ni exigenceUIDCalico=UIDK8s. Aucun nouveau probeCRD/IPAM n'est requis ou autorisé.
Une RVNode de heartbeat différente ne suffit pas à invalider un champ sémantique
stable ; changementUID/source/placement/incarnation/Service=>STOP/rollback possédé.
Aucune atomicité multi-objets n'est prétendue, aucune union d'IP anciennes/nouvelles.

## Patch, témoin et rollback

L'[amendement ACK ratifié](local-full-envoy-ingress-patch-ack.md) prime les seules
clauses de propriété same-object/RV ci-dessous : la réponse du patch, jamais un
GET ultérieur, attribue la révision possédée. Compatibilité de helper fermée et
qualification exacte après code obligatoires ; aucune paire future approuvée à blanc.

- Seulement allow-envoy UIDexact possédé. JSONPatch atomique teste metadata.uid,
  RVcourante et ancien spec, puis append d'une seule règle T/32TCP10000. Préserver
  intégralement peerclient, egress, selector, policyTypes et autres champs.
- Avant action : journal nonsecret privé des SHA/aviscausal/CPID/NodeUID/tunnelT,
  policyUID/RV/ancien spec/patch attendu. Preuve/revue absente, remplacée, incohérente
  ou sémantiquement périmée=>zéropatch. Pas de booléen d'autorisation à blanc.
- Règle déjà exactement présente=>no-op, pas de doublon. Autre exceptionCP ou
  ingress inattendue=>STOP, pas accumulation/fallback. Relecture UID/spec/source/
  Pod après mutation ; mismatch=>rollbackconditionnel.
- Validation bornée : même entrée et GET
  http://127.0.0.1:18080/iam/api/auth/gitlab/login, réponseIAM/nonce/redirect du
  connecteur attendu, pas TCP/HTTP5xx/directPod. Admin mêmeepoch/incarnation et
  compteursmonotones explicites. Ce témoin n'est PAS campagne navigateurP1–P3/N1–N3.
  Routes protégées sansBearer restent refusées, defaultdeny/client/egress/TLS/SAN
  inchangés. Témoin sourceeffective autrequeT seulement s'il est déjà disponible
  et possédé ; pas nouveauPod/proxy ou faux négatif nœudrésident.
- Si dérive, règle plus large ou absence d'effet ciblé : retirer la SEULE règle
  ajoutée avec testsUID/RV/valeurcourante, jamais restaurer toutspec aveuglément,
  adopter une policy remplacée ou effacer des modifications concurrentes. Rollback
  bloqué=>STOP/risque signalé, pasPASS. No-op ne donne pas propriété d'une règle
  préexistante. Un applyfutur peut retirer la règle : réconciliation avec les mêmes
  gates, jamais une adresse historique prise pour acquise.

## Allowlist et tests avant exécution

ops/deploy/deploy-local-full.py : fonction spécialisée du runner existant,
preuve+dérivationtunnel+patchconditionnel. Ne pas modifier configure_api_policy
(API/physiqueInternalIP), policies.yaml, renderer/Service/Calico/kubeproxy/Envoy,
helperautorité, app/SDK/Cargo/RBAC/configglobale. Métadonnéespubliques via garde
admis, pasSecret/env/inspectbrut ou nouveau transport. Besoin horsgarde=>remonter.

Tests fake ops/tests/test_local_full_envoy_ingress.py, noms avant run :
- test_envoy_ingress_requires_measured_tunnel_of_exact_owned_cp
- test_envoy_ingress_tunnel_mismatch_ambiguity_and_stale_proof_block_patch
- test_envoy_ingress_patch_tests_uid_version_and_old_value_and_preserves_peers
- test_envoy_ingress_noop_and_conditional_rollback_do_not_adopt_replacement

Suite unittest de ce fichier, pas de nouvelle infrastructure/harness. Infra-verifier
après changement réseau significatif, faits live distincts des mocks. Aucun runtime
par l'implémenteur ; parent applique seulement après tests/revue/sourcequalifiée.

## Sources et continuité

Original auteur55lignes v2 conservé au worktree pool-4. Sources : policies.yaml,
contrats0412/0606, kubeproxy1.32.2 (attenteMASQ, pasIPeffective), NetworkPolicy1.32
NAT, Calico3.29.7 lib/backend/k8s/resources/node.go (annotationIPIP↔BGP/OrchRef)
et APIv3node.go. RO21finalmsg_1876a00a20017bmDPHmX7cbBbN ; aucune mesure auteur.
La ratification et le jugement causal appartiennent au lead sous délégation,
pas à une acceptation autonome du document Proposed ou à un PASS du helper.
