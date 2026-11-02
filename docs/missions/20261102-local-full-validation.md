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
