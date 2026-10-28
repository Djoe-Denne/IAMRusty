# Rapport 09 — conflit de profil T1

## Objectif et état

T1 doit produire le contrat architectural avant T2. Aucun livrable écrit,
aucune commande de test ni ressource runtime démarrée par le membre.

## Faits vérifiés

pool-1 confirme profil natif build, modèle openai/gpt-6.1-sol xhigh. Il attend
une décision de profil. Ses entrées manquantes (AC1–AC7 et chemin absolu du
worktree) sont maintenant persistées dans ../t1-inputs.md et transmises.
La transmission a été stockée : le harness indique le membre terminé et ne
l'a pas réveillé. Cela n'atteste ni livraison du contrat ni validation de T1 ;
aucune réactivation avant l'arbitrage ci-dessous.

Retour final du membre récupéré après ce rapport : Status failed, aucun
fichier modifié ni commit, Cargo ou runtime. Ce résultat confirme le même
blocage ; sa mention des entrées absentes précède leur persistance dans
t1-inputs.md. Ne pas rouvrir ces demandes ni interpréter ce retour comme une
nouvelle tentative. La seule décision restante concerne le conflit de profil.

Le plugin actif index.js:14698 refuse explicitement le changement de profil
d'une tâche autonome : "An autonomous task cannot change its profile; use
phase/ reasoning instead". Un message approve/handoff ne lève pas cette règle.
Le canon projet réserve les décisions structurelles à architecte.

## Options et recommandation

1. Corriger le routage/admission dans le harness pour permettre T1 sous
   architecte, sans mutation manuelle de sa base ni override de la tâche admise.
   Respecte le canon, mais impose une résolution d'admission avant reprise.
2. Dérogation utilisateur explicite et limitée : autoriser build à produire
   le contrat Proposed de T1, sans ratification Accepted autonome et sans changer
   son modèle/profil natif. Plus immédiat, mais déroge au routage architectural.

Recommandation : option 1 pour conserver le canon. Ne pas entreprendre une
campagne de réparation du plugin sans mandat. La mission reste suspendue à T1,
sans ouvrir T2 ni ajouter de membre.

## Décision demandée

Choisir correction du routage/admission ou dérogation limitée au rôle build
pour la production du contrat T1. Les AC, le plan et la QA ne changent pas.
