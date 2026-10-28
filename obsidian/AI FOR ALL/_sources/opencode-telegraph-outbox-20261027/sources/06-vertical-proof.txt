# 139. Parcours complet validé au nominal et après panne

## Résultat vérifié

L'unique validation134 autorisée réussit : **1 PASS,0 échec,0 ignoré,6 filtrés**,
Cargo0/lanceur0,25,39s libtest. Le test comprend le nominal et la panne sortante
suivie d'arrêt/restauration/reconstruction sur la même base.

Deux marqueurs de preuve, nominal et panne, attestent l'événement et la
notification corrélés, statut published, tuple recipient unique, autre utilisateur
HTTP403 sans mutation, destinataireHTTP200 et notification lue. Le test passe par
le vrai message SQS et SyncEventHandler/OpenFGA sans droit préchargé ; replay des
mêmes bytes avec ledger neuf conservé. Les avertissements OpenFGA de tuple déjà
présent accompagnent le replay traité idempotemment ; ce ne sont pas des tests
ignorés ou erreurs masquées de ce run.

## Mesure temporelle et portée

Dans ce run, E stockée vaut19:07:56,128008Z, t0 vaut19:07:57,2256282Z : résiduel
nul, deadline fixée à25s. Les observations couvertes finissent266736µs après t0,
avantD. Il n'y a pas eu de lease encore active à la reconstruction dans cette
verticale. Le composant132 couvre séparément ce cas après expiration. Ne pas
prétendre avoir reproduit ou expliqué la course historique109 ; rouge conservé.

La revue statique indépendante et la compilation restent celles de137/138,
source80cce027 identique. Aucun produit, lease, backoff, IAM25s ou arrêt6s n'a été
modifié pour obtenir le vert. Pas de rerun supplémentaire ou campagne globale.

## Nettoyage recoupé après campagne

Nouveau relevé full-ID :4 créations,3 absentes,1 SMTP Exited(2) conservée ASK,
0 possédée active,0 nouvelle identité inconnue. Les15 conteneurs préexistants ont
gardé identités et états ; leurs138 volumes sont préservés,140 présents après
campagne,2 opaques nouveaux ASK. Le lanceur a correctement classé le SMTP ASK,
pas de correction destructive, prune ou suppression de volume. Desktop/WSL
conservés pour l'étranger actif ; RAM non mesurée.

## Suite autorisée

Le blocage fonctionnel de validation verticale est levé pour le périmètre testé.
Le mandat initial53 autorisait déjà le correctif SDKHTTP, ses tests et publication/
pin exacts. Cette livraison peut maintenant être préparée sans nouvelle étape
d'approbation artificielle, en vérifiant les refs distantes et en publiant le SHA
testé/revu sans force. Aucun push n'est encore affirmé dans ce rapport.

Clippy strict préexistant non vert et Sonar non exécuté restent des réserves,
pas un PASSglobal inventé ; les limites d'annulation externe/erreurs startup non
forcées restent documentées. ADR0313/0503Proposed,0301Partial inchangés.

## Traçabilité

Run `vertical134-06bd9d7e772c4370`, sourcesAI1f2dd30/SDKd60,
verticalSHA80cce027. Commande exécutée :
`cargo test --locked -p telegraph-service --test notification_created_routing_test user_email_verified_routes_same_event_through_sentinel_and_real_authorization_after_publisher_recovery -- --exact --test-threads=1`.
Archives privées TI/.tmp/vertical134-06bd9d7e772c4370 : log,summary,bail,fixtureledger,
inventairesavant/après et lead-cleanup-verification.json. Registre contemporain
docker-resources-20261027.log append-only. Localisation : verticale Telegraph.
Impact : preuve comportementale ciblée acquise. Limites : pas preuve de publication
SDK, pin livré ou CI/Sonar globaux à cette étape.
