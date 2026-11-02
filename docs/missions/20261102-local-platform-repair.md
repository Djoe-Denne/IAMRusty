# Réparation ciblée de l'ancienne plateforme locale

**État technique : Manifesto, OpenBao, Lazaret et operator sont Ready.** Les
contrôles de données/certificats sont conformes, y compris les recalculs live
indépendants. **Vérification indépendante : 7/7 critères PASS**, dans le périmètre
technique décrit ci-dessous, sans qualification métier/cloud implicite.

## Mandat

User **« go 1 »** : réparer Manifesto et l'ancien raccord OpenBao/Lazaret/operator,
sans reset, redéploiement global, remplacement de volume ou rotation des certificats
existants. OpenBao reste volontairement volatile. Les E2E auth/métier, le refresh
entre deux digests et la release cloud depuis un commit master exact restent séparés.

Sources exécutées : `master@3c7839a8eff8e226327a8c5d51b20d02cfa377b0`, SDK
`93b890ff7b821e9cb2e7d0aeb81957d098778a2f`. Les builds locaux capturent les octets
réels du checkout et du SDK ; ils ne sont pas présentés comme des releases cloud.
Deux correctifs durables supplémentaires sont nécessaires : retirer l'init root
de l'operator dans le profil local-full et réaligner conditionnellement le `/32`
API du control-plane à la reprise, sans refaire le déploiement.

## Manifesto : binaire ancien, configuration correcte

L'échec `PRINCIPAL_POLICY_UNAVAILABLE: Authorization policy is required` vient de
l'ancien binaire, qui n'alimente pas la politique dédiée du snapshot. Le code
courant `grant_snapshot_user_id_extractor` transmet déjà l'issuer plateforme et
`Some(false)`. La configuration déployée correspond aux sources : aucun
assouplissement de l'authentification ou workaround de configuration n'est ajouté.

Image reconstruite puis publiée par le registre existant :
`aiforall-apps-registry:5000/apps/manifesto@sha256:155bb965a06952d8785a2fb978e9796fda59ff6cea21c80b65879bf869f8418c`.
Patch CAS UID/resourceVersion limité à l'image et la politique de pull.
Pod `bbb1d445-b35d-4675-9ab2-70b14d8c2f82` Ready, zéro restart.

Les trois migrations courantes étaient déjà appliquées. Sauvegarde custom de
64 886 octets, table des matières relue par `pg_restore --list`. Après rollout,
schéma, ledger de migrations et empreinte complète des données sont inchangés.
L'entrypoint exécute toujours `/app/migration up` : le gate « aucune migration
en attente » et les comparaisons avant/après ne prétendent pas supprimer ce code.
Il s'agit d'une sauvegarde avec TOC vérifiée, pas d'une nouvelle restauration testée.

## Préflight OpenBao/Lazaret/operator

- IAM/Lazaret sont redevenus Ready après le démarrage, sans modification. Aucun
  connecteur GitHub ni fichier d'entrée GitHub n'est enregistré ; `connectors=[]`.
  Le snapshot HMAC IAM est non applicable à ce profil et IAM reste intact.
- Le coffre live utilise l'ancien DEV HTTP. Le wrapper existant dans les sources
  réalise le bootstrap localhost, génère un root éphémère, le révoque puis ouvre
  la disponibilité HTTPS8443. Le contenu du coffre demeure volontairement volatile.
- Extension PKI additive signée par la CA existante : les **50 fichiers déjà
  présents sont octet pour octet inchangés**, identité de confiance du registre
  revérifiée. La feuille GitHub produite par le helper commun n'est pas utilisée ;
  aucun service/provider GitHub n'est ajouté. Nouveau Secret TLS OpenBao par CREATE
  uniquement ; aucune lecture ou réécriture de Secret existant.
- Les anciennes images Lazaret/operator portent une révision ne supportant pas
  la CA privée. Deux images courantes reconstruites/publiées : Lazaret
  `sha256:3fe9102f81e46d926dec67ee2669790fa102f4b699314a83228559cc8898b696`,
  controller `sha256:60dc1a24d8dba4981a8159c069bb1b7cf78a6a70290d9c292b5fc3ce91359313`.
- Lazaret : trois migrations déjà appliquées ; sauvegarde custom22 849octets et
  TOC vérifiée ; empreinte du certificat serveur existant conservée comme oracle.
  Le `server-leaf` copie ce certificat depuis son Secret existant, sans rotation.
- L'operator n'a jamais créé de Pod. Ancien hostPath absent sur les trois nœuds ;
  PVC `admission-store` original Pending, sans PV. Aucune donnée hostPath à transférer.
  Son ancien init root `fix-store-perms` reste néanmoins incompatible avec le
  namespace restricted : suppression dans le seul overlay local-full, UID/GID et
  fsGroup65534 conservés, aucune relaxation de politique ou chown de données.
  Référence : [Pod Security Standards](https://kubernetes.io/docs/concepts/security/pod-security-standards/).

## Périmètre transactionnel et vérification

Le plan privé original est conservé ; une version2 retire seulement l'init root.
Cinq créations ConfigMap/ServiceAccount, un nouveau Secret TLS et sept changements
sur objets existants sont prévus : OpenBao Deployment/Service, trois NetworkPolicies,
Lazaret Deployment et operator Deployment. UID, resourceVersion et préimages sont
contrôlés avant chaque patch ; aucun rendu/apply global, aucune adoption d'objet.
Ordre : OpenBao → Lazaret → operator. Une interruption impose la lecture du
journal réel, pas le rejeu aveugle des CREATE.

Trois tests offline ciblés PASS (2,618s), comprenant les profils base/GitLab/GitHub :
absence d'init root, PVC/volumes auxiliaires conservés et identités OpenBao.
Pas de campagne Cargo/IT/E2E ajoutée pour ces rollouts de réparation.

Deux contrôles de postimage ont arrêté les scripts après des CAS pourtant
appliqués : valeurs par défaut Kubernetes ajoutées aux sondes/volumes OpenBao,
puis `readOnly:false` omis du JSON Lazaret. Les écarts exacts ont été archivés et
la comparaison corrigée uniquement pour ces règles explicites ; les erreurs
initiales restent enregistrées comme FAIL. Aucun CREATE ou CAS déjà appliqué
n'a été rejoué. Le nouveau Pod OpenBao est Ready, zéro restart ; le Service TLS
répond200 depuis l'ancien Lazaret **et** le nouvel init, avec CA et hostname
vérifiés, sans credential.

**Échec intermédiaire conservé : le rollout Lazaret a expiré pendant son login de workload.
Une seule reproduction technique bornée dans l'init existant a donné un timeout
au login, pas une erreur TLS ni un403. Diagnostic du hop OpenBao→TokenReview en
cours ; aucun token obtenu/persisté par cette reproduction.** L'ancien Lazaret
est resté Ready jusqu'à la résolution décrite ci-dessous. Le correctif de démarrage operator est indépendant : aucun
AdmissionRecord existant à réconcilier au préflight. Son rollout a réussi : Pod
`46a6594c-e7b2-4d42-98e7-90bf2ebca3f1`, zéro restart. PVC original
`d28b3502-822c-4940-a96a-02a4b8e0260b` Bound au PV
`pvc-d28b3502-822c-4940-a96a-02a4b8e0260b`. `test -w /var/lib/apparatus` réussit
sans init root, chown ni écriture de données de test. Ce succès ne prouve pas une
admission/signature métier.

Le premier diagnostic « DNS non autorisé pour OpenBao » a été **écarté** : la
policy live `allow-dns` sélectionne déjà tous les Pods et les autorisations sont
[additives](https://kubernetes.io/docs/concepts/services-networking/network-policies/).
Aucune règle redondante ou ouverture réseau globale n'a été appliquée.
La sonde DNS courte depuis OpenBao a réellement échoué ; le FQDN résout. Le
diagnostic final a distingué cet artefact BusyBox de la cause réseau réelle.
Documentation de référence : [OpenBao2.6 — Kubernetes auth](https://openbao.org/docs/2.6.x/auth/kubernetes/).

### Cause réelle et correction API

L'IP du control-plane sur le bridge Docker est passée de `172.19.0.3` à
`172.19.0.2` sans recréation du cluster. Docker, l'Endpoint Kubernetes, l'adresse
annoncée par l'API et son SAN concordent sur `.2`. L'ancien `.3` est désormais
un worker. Les deux `allow-api` possédées conservaient ce `/32` obsolète : après
DNAT du Service API, TokenReview visait une adresse non autorisée et expirait.
Le diagnostic différentiel a observé timeout vers la vraie API, refus immédiat
sur le worker autorisé et DNS FQDN fonctionnel. Aucune capture de paquets ou
inspection des chaînes Calico n'a été effectuée.

Équivalent CAS borné du helper d'installation existant : remplacement du seul
`172.19.0.3/32` par `172.19.0.2/32` dans `allow-api` de full **et** apparatus,
UID/RV/préimages contrôlés. ClusterIP, ports443/6443, sélecteurs et DNS conservés.
**Pas de restart OpenBao, de bootstrap rejoué ou de nouveau rollout Lazaret.**
Son init a repris naturellement : même Pod `13d9d614-c7ea-4c67-b1c0-c02fd8341f24`,
snapshot terminé exit0, processus applicatif Ready/0restart. Les six restarts
de l'init avant correction restent comptés, sans les masquer.

Égalité structurelle complète de la base Lazaret avant/après, trois migrations
inchangées ; certificat serveur SHA256
`730c01d4b6018df6a4e57954e299a2f5b43d84bb7b5d94c5681b876b74cb7d35`
inchangé. Epoch du coffre enregistrée seulement après ces contrôles dans l'état
stable, pas dans un nouveau bail. Aucune nouvelle tentative de login de diagnostic.

### Prévenir la récidive dans le parcours existant

`local_full_environment.resync_api_policies` s'exécute uniquement dans `up`,
après les nœuds Ready et avant la récupération du coffre. Il contrôle les
propriétaires UID et les deux formes de règle connues avant écriture ; chaque
patch conserve les tests UID/RV/spec. IP déjà correcte ⇒ zéro mutation ; forme
inattendue, adresse invalide ou propriété inconnue ⇒ refus. Aucun nouveau
framework, aucune règle DNS, aucun changement de workload.

**Huit tests offline ciblés PASS en0,362s** : changement d'IP, no-op, préimage
inattendue, IP invalide/ambiguë, intégration up/stop/status et cleanup sur échec
avant récupération. Appel réel du helper sur le live réparé : **zéro patch**.
Avec les trois tests overlay, onze tests ciblés ont été exécutés. Pas de nouveau
cycle power live pour forcer une variation d'IP : les nœuds étaient déjà running
avec provenance inconnue. Ce delta ne requalifie ni E2E métier ni cloud.

### Vérification indépendante intermédiaire

Même session `infra-verifier` que la tranche précédente, autorité du bail original,
sans modification ni nouvelle tentative de login : **six critères PASS bornés,
un INCONCLUSIVE (Lazaret)**. Ce n'est pas un succès global de la mission.

- Manifesto : UID conservé, exactement image/policy modifiées, digest CRI exact,
  Ready/0restart. Empreinte live `manifesto_dev` recalculée indépendamment en
  lecture seule : égalité structurelle complète des 22relations avec avant/après.
- OpenBao : UID conservé, Service/container8443 et endpoint unique concordants,
  wrapper/readiness corrects, Ready/0restart. TLS actif soutenu par les probes
  publiques archivées, pas de rejeu auth par le reviewer.
- Operator : chaîne PVC/PV/claimUID originale, non-root65534, aucun hostPath/init
  root, Ready/0restart ; `test -w` indépendant sans création de donnée.
- Préservation : 15/15autres Deployments UID/templates identiques, PVC
  PostgreSQL/Zot/Redis UID/spec identiques et Bound, 50/50anciens fichiers PKI
  recalculés identiques ; 29/29NetworkPolicies UID conservés, seuls trois
  changements de port autorisés. Limite : autres Services non diffés exhaustivement.
- Profil courant sans GitHub et anciennes ConfigMaps IAM/Lazaret conservées.
  À ce stade intermédiaire, l'epoch de l'état applicatif stable restait à finaliser ;
  le contrôle du lease seul ne prouvait pas cette égalité.

Les erreurs et probes INCONCLUSIVE précédents restent des erreurs/non-conclusions.
La vérification finale du certificat serveur et de la base Lazaret a ensuite été
finalisée dans le même reviewer, sans nouveau login ni mutation.
Trois images ont réellement été reconstruites/publiées dans cette mission ;
l'absence de build dans les seuls opérateurs de rollout ne signifie pas « zéro build ».

### Vérification indépendante finale et limites

Follow-up de la même session `infra-verifier` : C3 Lazaret et C6 epoch **PASS** ;
delta API et revue source **PASS**. Avec les critères déjà prouvés : **C1–C7 PASS**.

- Même Pod Lazaret `13d9d614…` : Running/Ready, app0restart, init6restarts puis
  exit0. Ancien Pod retiré par le scale-down standard du rollout, pas par un
  recyclage forcé ajouté. Certificat public recalculé identique au préflight.
- Base `lazaret_dev` recalculée indépendamment en transaction READ ONLY :
  égalité structurelle complète des sept relations avec avant/final. Manifesto
  avait déjà le même contrôle indépendant sur ses22relations.
- `coffer_epoch(target)` recalculé depuis les identités publiques du Pod/container
  OpenBao égale `environment.json.epoch` et la preuve finale.
- Les29UID de NetworkPolicy sont conservés ; **exactement cinq deltas** : trois
  ports8200→8443 et les deux CIDR API. Aucune apparition/disparition de policy DNS.

Risque borné du correctif de reprise : les deux CAS ne sont pas une transaction
multi-objets. Si le second échoue par édition concurrente, le premier peut rester
appliqué ; l'erreur est remontée et le prochain `up` recontrôle les deux formes.
Une forme inconnue impose l'inspection, pas un rollback ou overwrite aveugle.

Les hypothèses DNS et un faux négatif du dict de comparaison du reviewer ont été
corrigés sur preuves, sans modifier le live pour satisfaire le comparateur.
Pas de nouveau cycle complet up/stop/reprise pour cette source, pas de test
admission/signature, de parcours auth/métier, de refresh inter-digests ou de cloud.

## Preuves et ressources

Archives owner-only hors Git :
`C:/Users/djden/.aiforall-local-repairs/20261102-2ba0879289ad/`.
Reçus réels `manifesto-image-build.json`, `manifesto-image-release.json`,
`openbao-consumers-image-build.json`, `openbao-consumers-image-release.json`, backups/TOC, empreintes DB et PKI,
plans privés et journal transactionnel. Aucune clé, configuration brute ou donnée
métier publiée. Opérateurs ponctuels sous `.tmp/`, pas de nouveau framework livré.

Trois nœuds full déjà running avant cette phase, provenance du démarrage inconnue :
aucun start/restart/stop de nœud attribué à ce lot. Le nœud local partagé reste
hors périmètre. Volumes, PKI, caches de compilation et images de retour arrière
doivent être conservés. Registre de ressources : `.tmp/docker-resources-20261102.log`.
Fin vérifiée : registre applicatif original `12d9ff21…` restauré **exited** ;
trois helpers nouveaux enregistrés à la création, exit0, conservés arrêtés.
Inventaire12→15 : les12ID et états préexistants sont identiques ; seuls les trois
helpers exited s'ajoutent. Aucun retrait/prune. Images, volumes, PKI et caches
conservés ; bail de réparation libéré. Les trois nœuds full et le nœud local
partagé déjà running restent intacts. Aucun shutdown global Desktop/WSL, delta
RAM non mesuré : propriété du runtime préexistant non résolue.

Certains anciens registres ont un encodage ou format nonJSONL : cette limite
interdit une suppression aveugle, pas une raison de les réparer hors mandat.
Preuve de sortie : `final-runtime-release-proof.json`.

Intégration dans `master`, sans branche/worktree/équipe créé :82branches et
cinq arbres Git/natifs préexistants conservés. UI Obsidian étrangère intacte.
La référence de publication vérifiée est donnée dans le retour de mission et
le ledger local ; ne pas la confondre avec la base des trois images locales.
