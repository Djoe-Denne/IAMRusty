# Rapport 14 — extension framework autorisée pour AC7

## Décision utilisateur

L'utilisateur autorise l'option 1 du rapport 13 : correctif cancellation-safe
limité dans rustycog-events, tests ciblés puis intégration du pin. AC7 n'est
pas réduit à l'arrêt coopératif. QA proportionnée et autres critères inchangés.

## Application au plan

L'architecte T1 a reçu l'arbitrage pour l'intégrer au contrat avant tout code.
T2 recevra un sous-lot préalable framework/tests/pin, exécuté avant l'atomicité
Telegraph. T3 vérifiera la supervision standalone/monolithe et réutilisera les
preuves valides. Aucun changement d'IDs ou de profils ni réécriture native des
descriptions n'est prétendu ; le mandat utilisateur et le contrat priment sur
l'ancien interdit framework pour cette extension seulement.

## Bornes et oracles demandés

Cycle de vie des pollers SQS, sans refonte des transports ni nouveau harness.
Tests ciblés : handler async bloqué, annulation du parent, erreur/panique enfant,
stop pendant longpoll et arrêt nominal ; observer la terminaison/destruction
des enfants, pas seulement celle du parent. Le contrat chiffre les échéances
QA et distingue l'annulation async du code synchrone non coopératif.

Préserver les changements préexistants du submodule et du parent. Pas de
commit/stash/reset utilisateur implicite. Un seul Cargo Windows, jobs configurés
à 24. Aucun changement global ou de modèle ; aucun code, pin, test ou runtime
modifié/lancé par cette autorisation seule. Livraison T1 encore attendue.
