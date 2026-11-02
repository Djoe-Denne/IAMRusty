# Demande d'exécution E2E sur le local-full réparé

## Résultat et périmètre

Demande utilisateur : lancer les end-to-end existants pour qualifier les parcours
après la réparation locale. Source initiale `c10f30e13f657d98f7d19559717dfb5056383e81`,
SDK `93b890ff7b821e9cb2e7d0aeb81957d098778a2f` ; correctif borné du lanceur décrit
ci-dessous, empreintes des octets exécutés archivées.

- **Disponibilité : PASS LIVE,21/21 contrôles.** Trois namespaces,17Deployments
  requis et vérification des PVC du namespace full. Cela ne valide pas les parcours.
- **Matrice publique : BLOCKED,0/33cas complétés.** Timeout sur la première
  requête à `http://127.0.0.1:18080/iam/.well-known/jwks.json`. Une requête tentée
  n'est pas un cas validé. Le diagnostic ci-dessous localise le blocage avant Envoy.
- **E2E authentifiés/métier : BLOCKED / NOT RUN,0test exécuté.** Ne pas agréger
  les21contrôles de disponibilité en PASS E2E ni attribuer un échec métier au cluster.

## Pourquoi les suites existantes ne sont pas simplement lancées

`just kind-full-e2e` expose bien une CLI, mais
`ops/tests/local-full-acceptance.py::explicit_probe_action` refuse encore
**inconditionnellement** `prepare-probes` et `e2e`, même avec un JSONv1 valide :
préparation full et sondes à UID attesté non livrées. Aucun input privé ou
credential n'est fabriqué pour contourner ce garde ; aucune suite métier n'a été
effectivement démarrée. Le diagnostic est fondé sur la source et son test de garde,
pas sur un faux run E2E avec de faux inputs.

Les suites historiques existent mais ne constituent pas un repli transparent :

- `ops/scripts/mesh-authn-kind-e2e.sh` fixe le contexte legacy et ses namespaces,
  crée des Pods/ConfigMaps et prépare OpenFGA. Il n'est pas lancé ni retargeté.
- `apparatus_m6_e2e_chain.rs` utilise une fixture Kind, build/load des images,
  démarre son stack de signature/registre et remplace des objets de test. Son
  contexte par défaut est legacy ; ses collaborateurs ne sont pas tous les
  services standalone déployés ici. Ce n'est pas une observation du full courant.

La suite nécessaire est un raccord des parcours existants à ce profil et à des
données/sondes isolées possédées ; ce développement n'est pas glissé dans le
mandat « lancer les tests ». Pas de nouvelle pile ou de redéploiement global.

## Défaut du lanceur trouvé et corrigé

Le premier lancement des deux modes publics s'arrêtait avant toute sonde :
`ModuleNotFoundError: private_diagnostics`. Le module est local au dépôt ; le
script autonome chargeait l'ancre par chemin fixe mais n'ajoutait pas son dossier
de dépendances à `sys.path`.

Correctif : résolution de `ROOT/ops/deploy` avant le chargement de l'ancre, sans
configuration Python globale ni installation. `ImportError` est désormais
BLOCKED avant sonde, pas un faux échec LIVE. Les sorties originales qui comptaient
artificiellement un échec sont conservées ; elles ne prouvaient aucun contrôle
de service exécuté.

Quatre tests ciblés couverts : import autonome sans PYTHONPATH, comptage0sonde
sur dépendance manquante, confidentialité des erreurs et maintien du garde E2E.
Trois ont passé à la première passe ; le test d'import a été corrigé de `-I`
(qui excluait aussi PyYAML installé en user-site) à `-E` (ignore PYTHONPATH), puis
rejoué seul avec succès. Aucun changement global de l'environnement Python.
Les échecs initiaux restent archivés.

## Exécution et preuves

Commandes canoniques, séquentielles sous le mutex du bail original :

```powershell
python -B ops/tests/local-full-acceptance.py smoke --environment <descripteur-v1>
python -B ops/tests/local-full-acceptance.py route-matrix --environment <descripteur-v1> --cases ops/tests/mesh-public-route-cases-local-full.json
```

Le descripteur publicv1 est construit avec le bail original vérifié. Il est
distinct de l'état lifecycle stable et ne remplace pas l'autorité du bail.
Les deux réponses422 OAuth prévues par la matrice signifient connecteurs désactivés,
pas logins réussis ; elles n'ont pas encore été atteintes dans cette exécution.

Archives owner-only hors Git :
`C:/Users/djden/.aiforall-local-validations/20261102-bd3e1e0e65e8/`.
Inventaire avant, descripteur, journaux en octets/SHA256, résultats initiaux et
`after-driver-fix/validation-results.json`, quatre tests ciblés. Aucun corps de
réponse, cookie, header ou credential enregistré par la matrice canonique.

## Diagnostic RO de l'entrée HTTP

Le spécialiste Envoy a constaté : TCP hôte accepté rapidement, puis zéro octet
et timeout ; pendant la requête bloquée, aucune connexion active sur les workers
du listener et aucune requête HTTP/IAM enregistrée. Admin loopback `/ready` sain,
listener10000 et routes chargés, cluster IAM présent et endpoint annoncé sain.
**Le trafic client n'atteint pas Envoy.** La santé TLS/SNI réelle d'Envoy vers IAM
n'a donc pas été exercée par cette matrice.

`allow-envoy` n'autorise en ingress que le Pod portant le label
`app.kubernetes.io/name=local-full-client` sur10000. Aucun tel Pod n'existe et
aucune entrée source hôte/nœud n'est déclarée. Avec le default-deny, l'entrée
NodePort hôte n'est pas autorisée à la frontière du Pod. Les probes Ready TCP du
nœud passent malgré cette frontière : un Pod Ready ne prouvait pas cet accès.

Limite : pas de capture node-level/conntrack/iptables ; un défaut de forwarding
supplémentaire n'est pas totalement exclu. L'IP source exacte après SNAT doit
être mesurée/attestée avant toute autorisation `/32`, pas simplement déduite de
l'adresse du control-plane. Aucune ouverture `/16`, suppression de policy ou
désactivation TLS n'a été faite.

Le seam existant `configure_envoy_ingress` n'est **pas** une commande à rejouer
aveuglément : il est volontairement hors du deploy/CLI, exige une revue causale
ratifiée et une source mesurée, puis rend une validation/rollback au parent. Ces
préconditions ne sont pas fabriquées à partir d'anciens dossiers privés. Le profil
courant a des connecteurs OAuth désactivés : un witness historique303 ne peut pas
être transposé sans vérifier son contrat face au422 attendu ici.

## Condition de reprise

Deux raccords distincts restent à autoriser/préparer :

1. Permettre l'entrée locale mesurée avec une règle strictement bornée et son
   witness, sans ouvrir une plage globale, puis rejouer les33cas publics inchangés.
2. Raccorder les parcours E2E existants au full et à des données/sondes isolées
   possédées, sans basculer sur les anciens environnements ni créer une nouvelle pile.

La demande de lancement n'est pas transformée implicitement en ce développement
et en cette modification réseau. Les E2E métier restent **NOT RUN**.

## Préservation, cleanup et livraison

Aucun Cargo/build/IT, nouveau runtime, seed ou Secret GET lancé. Diagnostic Envoy
strictement RO ; deux port-forwards admin éphémères fermés, admin jamais exposé.
Aucun YAML/ConfigMap/NetworkPolicy modifié. Aucun conteneur/image/volume créé par
ce lot ; aucune nouvelle ligne de création Docker n'est à ajouter. Le registre
applicatif reste exited, les trois nœuds full et le nœud local partagé préexistants
restent intacts. **15ID/états Docker avant/après identiques**, aucun retrait ni prune.
Desktop/WSL non arrêtés, RAM non mesurée ; bail de validation libéré.

Source durable limitée au lanceur, ses deux régressions et documentation. Aucun
nouveau framework, branche, worktree ou équipe ; UI Obsidian étrangère préservée.
Intégration/publication sur `master` tracées dans le retour et le ledger local.
Cette livraison ne transforme pas les blocages de parcours en qualification.

## Suite autorisée le 2 novembre : raccord au déploiement existant

Le GO utilisateur « oui » autorise désormais les deux raccords. Le
[contrat opérationnel](../contracts/local-full-existing-e2e-wiring.md) conserve les
oracles mesh39 et Apparatus6, la préparation/exécution/nettoyage séparés, les
identités et les données préexistantes. Aucun changement applicatif Rust, SDK,
migration, image ou manifeste de déploiement n'est inclus dans ce raccord.

### Correction de l'hypothèse de profil

La vérification du **fichier effectivement monté** dans IAM contredit ici la
lecture du seul fichier base `config/iam.toml` : le runtime existant est
`oauth-proof`, avec **GitLab** activé, GitHub absent. Le parent a qualifié le
montage et son empreinte, puis projeté le seul identifiant du connecteur ; aucun
Secret GET, dump de configuration ou export de clé. L'ancien diagnostic de
timeout demeure vrai ; les attentes422 GitLab du fichier base n'étaient en
revanche pas adaptées à ce runtime. Elles ne sont pas réécrites rétroactivement.

Le témoin retenu pour cette activation est donc le **v1/303 strict existant**,
avec nonce/cookie/redirect et refus sans Bearer, pas un422 promu en succès. La
capacité base-full distincte reste testée offline et inutilisée sur ce profil.
La variante publique versionnée
`mesh-public-route-cases-local-full-oauth-proof.json` conserve les33IDs/méthodes/
routes/corps ; seuls GitLab loginHEAD303 et relinkHEAD400 sont déclarés selon la
source du profil. Une correction supplémentaire explicite du callback GitHub
désactivé attend422 : `oauth_callback` résout le connecteur avant le contrôle du
state ; `oauth_connector_not_configured` est mappé422. L'échec initial sur cet
oracle400 est conservé, pas réécrit. Les noms historiques restent des clés de
corrélation. Les deux fournisseurs inconnus restent attendus401/403.

### Entrée mesurée et état intermédiaire

La nouvelle capture native mesure **10.244.230.64/32**, tunnel IPIP du CP, et non
son IP bridge172.19.0.2 : binding conntrack unique551416958, delta deny actif8/480,
passif0/0, compteurs IAM/TLS inchangés. Le compteur demeure partagé par chaîne,
sans attribution par paquet. Revue causale parent, sept projections readonly,
contrôle des identités et revue sécurité indépendante sont conservés hors Git.

La première tentative d'activation a été **refusée avant journal, ACK et patch** :
la policy `allow-bao-seed`, possédée par le bail original, était absente du
catalogue source courant. Elle est préservée, non supprimée. L'amendement borné
reconnaît uniquement son nom et sa spec historique entière, avec l'ownership
original, sans accepter d'autre policy inconnue. Les22tests purs correspondants
(20ingress et2profils publics) passent ; les échecs intermédiaires sont archivés.
La seconde activation et ses preuves live restent à consigner ici. Ce point
intermédiaire ne signifie ni entrée rétablie ni E2E PASS.

**Seconde activation : PASS LIVE, entrée rétablie.** Après revue du catalogue
exact et22tests purs PASS, le parent a ajouté seulement
`10.244.230.64/32 TCP10000` par le seam existant, avec journal avant mutation et
ACK immuable. Les témoins réels sont JWKS200 avec clé publique RSA utilisable,
GitLab303 avec nonce/cookie/PKCE/destination validés en mémoire et `/iam/api/me`
sans Bearer403 sans cookie ni redirection. Aucun cookie/state ni clé privée
n'est archivé. IAM upstream progresse de2 ; le refus protégé ne provoque pas
de dispatch supplémentaire. Compteurs d'erreurs TLS inchangés, identités et
profil identiques avant/après, `finish` v1 accepté. Aucun rollback nécessaire.
Cette preuve établit l'entrée et le refus, pas un login vendor ou les39/6oracles.
La vérification infra indépendante et les33cas publics sont alors lancés ; leurs
résultats restent séparés des parcours métier.

**Infra indépendante : PASS borné.** La policy live est exactement l'ACK41542,
les24autres policies sont identiques en UID/RV/spec ; les25noms, les identités
Envoy/IAM et leurs restarts5/4 préexistants sont conservés. Les17Services et
Endpoints ont leurs adresses ;15DockerIDs/états avant/après identiques. Le
vérificateur n'a lancé aucun HTTP ni ouvert l'admin ; ses constats ne remplacent
pas les témoins parent.

**Matrice publique : toujours FAIL, pas un PASS de33cas.** Le fichier base
inchangé s'arrête à la quatrième tentative (GitLabHEAD303 au lieu de422). La
variante GitLab corrigée du callback désactivé s'arrête à la seizième :
**15PASS,1FAIL,17BLOCKED**, bilan33 conservé. Un diagnostic auxiliaire complet
montre deux désaccords sur unknown-provider login/callback :422 au lieu de401/403.
Ils restent rouges. Les33requêtes de diagnostic ne sont pas renommées33oracles
PASS et leurs corps/cookies/headers ne sont pas archivés.

Revue sécurité : hardening préexistant, non bloquant sécurité ; la regex publique
Envoy `[^/]+/(login|callback)` admet le dispatch IAM pour un slug inconnu. IAM
refuse avant transaction, échange ou credentials : aucune élévation ni accès
métier indu démontré. La correction des exemptions exige un lot configuration
distinct et une liste publique explicitement décidée ; aucun manifeste n'a été
modifié pour masquer ces échecs dans le raccord.

### Adaptateur et garde avant création métier

Le raccord mesh39 est livré en worktree pour revue, Apparatus6 reste un
inventaire **INCOMPLETE**. La revue correctness a bloqué avant signup : reçu
nettoyé réutilisable entraînant un faux PASS cleanup, prérequis GitHub enregistré
non vérifié, notifications/deliveries Telegraph indirectes absentes du cleanup.
Une revue sécurité ajoute la garde d'empreinte entière du provider dans l'inverse
atomique. Les corrections sont en cours ; **aucun compte ou fixture métier créé**.

La capacité interne `e2e_forward` est ajoutée **après** clôture ingress, sans
modifier sa preuve historique ni ajouter de pin de compatibilité. Cibles fermées
IAM/Hive/Telegraph8443 et MailHog8025, Pod/Service/owner/EndpointSlice/UID attestés,
loopback, marqueur exclusif et fermeture du seul enfant possédé. Après63tests
purs et revue source, un préflight réel a refusé l'init IAM `oauth-proof-trust`
avant spawn. Cet init déjà déclaré est désormais admis uniquement pour IAM,
terminéexit0 avec incarnation vérifiée ; PodReady explicite et64tests purs PASS.
La revue du delta a donné GOsource borné, sans GO métier.

Le second préflight réel passe pour **les4tunnels, tous fermés et marqueur absent**.
Une recherche MailHog vers un destinataire UUID inexistant répond200/total0.
Aucune clé lue, aucun compte créé, aucun objet Kubernetes ou conteneur créé ;
cela ne prouve ni les appels internes mTLS ni la NetworkPolicy depuis un Pod.

Les amorces OAuth303 constituent de l'état protocolaire normal côté serveur,
pas des GET garantis sans effet : cinq amorces observées dans le témoin et les
replays/diagnostic publics de cette suite. Aucun cookie/state conservé ou login
abouti ; expiration applicable mais purge physique non qualifiée. Ne pas
annoncer un état DB global inchangé, supprimer par date ou adopter des PK non
ACKées. Ce bilan est distinct du nettoyage des futures fixtures métier.

La fixture `SIMULATED_PROVIDER` ne nécessite pas de token GitHub vendor, mais
n'annule pas `require_registered_provider` des handlers S2S : le profil GitLab
seul ne satisfait pas ce prérequis. La disparition supposée d'un token personnel
ne cause donc pas ce blocage. `gh auth status` a par ailleurs réussi, sans lire
ou afficher de token ; aucune App, connexion interactive ou credential créée.

Archives de cette suite :
`C:/Users/djden/.aiforall-local-validations/20261102-wiring-e877d9eb8a4c/`.
