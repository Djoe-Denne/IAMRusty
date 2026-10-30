# Enquête documentaire bornée : ce que prouve l'isolation Windows observée

Architecte12 existant, même session/profil/modèle ; lecture et avis uniquement.
ContratsWindows fonctionnel et versionAccepted, révisionnative2 ; l'APIversion est
ratifiée, pas besoin de la redécouvrir. Le reviewer21 a terminé sa revuefigée68631c0
et a soulevé R-2 dans `20261030T1913Z-security-k6-windows-primitives-cbcc1bb.md`.

## Question technique précise

RendererToken lit primaireTokenRestricted/Untrusted/SIDnull/privilègesactivés
absents puis un booléenstickypositif sur n'importe quel renderer. Le reviewer
recoupe Chromium156sandbox.md : le primairelockdown précède LowerToken(), tant
que le thread initial conserve l'impersonation pluspuissante. Pas de preuve de
fin effective de bootstrap ou liaison du renderer ayant exécuté l'unique cas.
Ce code peut donc seulement prouver des restrictions primaires échantillonnées.

Déterminer si une preuve **bornée, non intrusive et réaliste sous compte ordinaire**
peut répondre au contrat existant avec le même cas about:blank, le parent existant
et les APIWindows normales. Vérifier sémantique des groupes/privilèges/token/threads,
moment d'observation, liaison au cas et droits requis. Ne pas remplacer une absence
de preuve par une hypothèse de succès après un délai750ms ou un jobde supervision.

## Limites et arbitrage demandé

- Aucun runtime, test, navigateur, WSL/Linux, acquisition, sourceOPS ou permission
  modifiée ; pas OpenThreadToken réel pour voir si ça passe. Rechercheofficielle
  Chromium156 et Windows, sourcesversionnées quanddisponibles, APIk6v2.3.0 siutile.
- Pas nouveau superviseur/protocole/instrumentation FATAL, portCDP/debug exposé,
  injection/extension de navigateur, capture privée, privilège/ACL ou pinchange.
- Si une méthode satisfait les frontières et la proportionnalité, expliquer le
  pluspetit raccord et oracles positifs/négatifs, contraintesréelles et limites.
  Si cela nécessite une nouvelle instrumentation/frontière, le dire, ne pas la
  déclarer implicitement admise. Pas nouvelle campagne/harness parfacilité.
- Si la preuve requise n'est pas accessible dans ce mandat, recommander
  explicitement ChromiumINCONCLUSIVE même si l'unique cas version/JS réussit,
  comme le critère accepté le prévoit. Distinguer nonqualification de sécurité
  runtime désactivée : conserver le sandbox Chromium actif, aucune relaxation.
  Peut-on réaliser le cas déjà autorisé avec verdictnonqualifiant après fermeture
  de S-1/R-1, ou existe-t-il une raison de sécurité concrète qui interdit même
  cette exécution ? Ne pas prendre un manque de preuve pour une évasion démontrée.

Les deuxautres findings de créationexclusive et intégritécode sont en correction
par auteur20 ; ne pas les rediagnostiquer. Lancementréel gelé jusqu'à disposition
revue, corrections/recherche en parallèle autorisées. Rendre avisnatif substantif
avec sources/LocalisationImpactLimites. Aucun nouveau contrat requis si simple
constatlimite ; un petit Proposed seulement si décisionde frontière utile, pas
modificationAccepted ouQAJSON. Lead décide dans le mandat, pas nouveauGOdejalon.
