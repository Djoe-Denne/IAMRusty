# 136. Préserver la jointure des tâches pendant la reconstruction du test

L'objectif est la validation complète autorisée en134, avec une échéance fixe
fondée sur la réservation SQL conservée. L'auteur a vérifié un risque du test :
annuler simplement `RunningApp::start` laisse son guard demander l'arrêt des
parents, mais ne permet plus au caller d'attendre leurs handles perdus.

Le lead valide donc un détail privé du seul test vertical : construction de
l'application avant les parents, puis conservation explicite du vecteur de tâches,
du guard et du propriétaire HTTP. Utiliser les surfaces publiques existantes
pour démarrer, arrêter et joindre. Sur échec ou expiration, cleanup6s avant
retour/panic ; sur succès, garder l'owner pendant les vrais contrôles HTTP et
l'arrêt final. Aucun changement production, fixture partagée, dépendance, API ou
nouveau backend/harness. Le parcours nominal conserve RunningApp.

Le cas déjà published utilise un résiduel nul sans inventer une nouvelle échéance
SQL ; vrai message SQS, identités et tous les oracles restent obligatoires.

L'auteur produit d'abord un candidat et un delta depuis6ac7 sous .tmp seulement.
Le lead inspectera et intégrera avant compilation canonique et revue indépendante
bornée au timing/ownership. Aucun Cargo auteur ni run à cette phase. L'unique
validation134 reste non consommée. Le contrat source a été amendé avant le GO.

Preuves : plan auteur pool23 et préflight hash/relevé Git sous
TI/.tmp/vertical-final134-inputs ; corps fixture34–62/runtime28–45/SDKstartup
effectivement examinés, source canonique inchangée. Limite : l'analyse est
statique, pas encore une nouvelle preuve comportementale.
