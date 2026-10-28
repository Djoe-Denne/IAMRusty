# Rapport 83 — faisabilité C-3, dépendance production minimale pour C-2

pool12 a lu les sources exactes : C-3 réalisable avec le client Hyper H2 déjà utilisé dans le module tests, sans ajout de dépendance/feature. Plan : synchroniser SETTINGS, constater annonce, envoyer Extended CONNECT réel et observer méthode/version/protocole/réponse router ; cleanup avant assertions. Aucun résultat encore exécuté.

C-2 : TelegraphApp::run lie inline son owner, App::new exige DB ; host construit aussi toute la plateforme avant le listener. Pas de seam existant pour le vrai caller sans cette plateforme. Worker propose un petit helper production recevant router/config et retournant OwnedHttpServer, effectivement appelé par run et host. Sources lead confirment host98 transmet config brute. **Direction acceptée, pas helper test-only ni copie de normalisation.**

## Séquence justifiée par les entrées et ownership

1. Pool12 continue tests C-3 et trois assertions RunningApp0/2+oracleHTTP possédé/joignable ; pas de production, IT ou C-2factice. Livrer une première remise distincte avec C-2 déféré pour prérequis production explicite.
2. Après intégration de cette remise, extraction production minimale du startup commun dans telegraph_setup::runtime, utilisée standalone et host, **d'abord sans changer la résolution du port**. Elle rend le vrai chemin accessible à un témoin loopback, sans introduire un nouveau serveur/harness.
3. Seulement après intégration de l'extraction, témoin C-2 consommateur : expected=config.actual_port(), startup communport0, GETexpected/arrêtjoint. Montrer le rouge avant normalisation si les préconditions le permettent. Correction caller nonTLS ensuite, port0/local_addrs owner direct et TLS-only/dual-bind inchangés.

Le correctif C-3 reste derrière intégration de son témoin ; aucune correction de protocole/port ou extension structurelle effectuée ici. Décisions d'architecture du contrat inchangées, détail local de factorisation startup seulement. Pas de nouveau feature/deps/harness/API SDK ou garanties réduites. C-1 demeure clos ciblé, C-2/C-3 ouverts, publication/pin bloqués. Cette séquence évite consommateurs d'entrées non intégrées et écritures concurrentes de runtime.rs.
