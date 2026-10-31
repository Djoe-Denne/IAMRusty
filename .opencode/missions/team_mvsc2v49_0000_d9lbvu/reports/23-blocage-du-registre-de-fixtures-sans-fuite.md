# Blocage du registre de fixtures, sans fuite de conteneur

La nouvelle reprise IAM s'arrête avant les tests d'admission, cette fois dans le
démarrage des tests de sessions : le harness ne peut pas créer son fichier de
registre exclusif. Ce n'est pas une preuve d'échec des sessions ni des quotas.
Le correctif du récepteur d'admission compile, mais n'a pas encore été exécuté.

Les27 conteneurs créés ont tous été retirés. Les20 conteneurs préexistants ont
conservé leurs identités et états ; aucune ressource inconnue ou fuite détectée.

Le registre utilise actuellement seulement l'identifiant de campagne et le
numéro de processus. Windows peut réutiliser un numéro après la fin d'un
processus : une collision entre deux binaires successifs est donc une hypothèse
précise à vérifier. L'erreur actuelle ne fournit pas sa catégorie système, donc
je ne présente pas encore cette hypothèse comme la cause démontrée de cet échec.

Je localise cette frontière avant de relancer. Aucun registre ne sera supprimé,
réutilisé ou tronqué pour forcer les tests. Une éventuelle correction restera
dans le mécanisme privé existant et conservera les garanties de propriété ; pas
de nouvelle infrastructure de tests ou de modification des règles de sécurité.

## Références techniques

- Source e44cbc8 ; campagne it-d36bd50882cc435eb036080642bdfe4a.
- Log privé `.tmp/auth-iam-admission-resume-a370908c9b194988a9112debe67af59c.it.log`.
- Frontier : rustycog-testing/src/common/fixture_runtime.rs186–191, ouverture exclusive du ledger ; erreurs lors du setup de session_persistence.

### Investigation bornée engagée

Un auteur travaille en lecture seule dans un worktree SDK isolé pour proposer
une correction minimale et des oracles déterministes. Il doit aussi vérifier le
test enfant qui consomme le nom du fichier : ajouter une identité d'incarnation
ne doit jamais conduire à lire le registre d'un ancien processus.

Aucune modification de production, nouveau harness, relance Cargo ou publication
SDK n'est autorisée à cet auteur avant validation du plan par le lead. Les
préautorisations de mission restent inchangées ; pas de nouveau GO utilisateur.

### Progrès indépendant vérifié

Les deux suites HTTP d'ext-authz passent :2 tests sur la révocation, le cache vide,
la borne réelle de60 secondes et la récupération ;11 tests sur les métadonnées
de confiance et le traitement des en-têtes. Aucun test ignoré, aucun conteneur
créé. Ces preuves ne remplacent pas Envoy réel ou le navigateur.

### Plan de correction du registre approuvé

Le plan reste limité à deux fichiers du SDK : donner au registre une identité
d'incarnation distincte du numéro de processus, tout en conservant la création
exclusive ; adapter son test enfant pour ne sélectionner qu'un unique nouveau
fichier depuis un inventaire pris avant le démarrage. Aucun registre ancien
n'est adopté. Le contenu des preuves et les règles d'identification des conteneurs
restent inchangés.

Sept oracles déterministes sont inscrits au plan avant exécution : nouvelle
incarnation, refus d'écrasement, diagnostic non secret et sélection stricte du
registre enfant. L'auteur dispose maintenant du seul créneau Cargo pour ces
tests purs ; aucun Docker, enfant de test, commit ou push ne lui est autorisé.

### Candidat vérifié, publication encore en attente

La régression de nommage est reproduite en rouge, puis les sept nouveaux tests
passent. La revue de sécurité ne relève aucun défaut sur le candidat exact et
ses règles de propriété. La revue de couverture reste attendue. Aucun commit,
push ou nouveau pinSDK à ce stade ; le test enfant Windows réel et la reprise
IAM restent non exécutés.
