# Démarrage du long run — contrats et identité TLS des serveurs

## Objectif et étape

L'utilisateur a approuvé la révision 1 de la politique de sécurité de l'authentification, avec préautorisations. Le long run est actif : pas de nouvelle demande de GO pour les étapes déjà couvertes. Les revues, ratifications explicites par le lead et barrières d'intégration restent des contrôles obligatoires, pas des questions systématiques à l'utilisateur.

Le premier lot traite la séparation des autorités dans les permissions. Les autres départs indépendants sont les contrats du signataire et du navigateur, ainsi que la correction des noms de serveurs TLS dans Envoy. Les dix consommateurs du plan restent non publiés jusqu'à intégration de leurs entrées.

## État vérifié au démarrage

- Politique native approved v1, long_run/running ; classification non requise pour les quatre racines.
- Le mécanisme natif a déjà affecté le contrat d'identité à un architecte isolé. Le lead réutilise ce membre et lui a livré le complément de contexte et la délégation approuvée ; aucun doublon de cette tâche.
- Les trois autres racines ont ensuite été lancées une à une par le mécanisme natif : contrat du signataire, contrat navigateur et noms des serveurs Envoy. Les quatre branches ont leurs worktrees propres ; aucun modèle/variant/score ajouté au spawn. Le catalogue a choisi Sol xhigh pour les trois admissions explicites.
- Base applicative et pin SDK inchangés depuis l'exploration. Les seuls états de travail étrangers constatés restent Obsidian, la configuration Serena du SDK, les worktrees et les artefacts de préparation ; ils ne sont ni committés ni nettoyés par cette phase.
- Cargo reste configuré avec 24 jobs et target-dir E:. Aucun processus Cargo ou runtime lancé par la mission.

## Ownership et parallélisme

Les trois contrats ont des chemins distincts. Si une ADR forward est nécessaire, numéros réservés après vérification d'absence : 0314 pour identités/permissions, 0315 pour transport remote, 0413 pour preuve navigateur. Les auteurs gardent Statut Proposed ; le lead détient l'index ADR, la ratification et le ledger. Réserver un numéro n'exige pas de créer une ADR inutile.

La correction Envoy possède les deux configurations mesh et ses tests statiques dédiés ; elle ne modifie pas l'entrée navigateur. Le futur consommateur navigateur attend son intégration avant de reprendre ces fichiers. Aucun runtime pour cette phase ; la preuve TLS comportementale reste différée à la campagne autorisée.

Les contextes sont fresh et les worktrees isolés selon les tâches importées. Les artefacts lead non suivis sont disponibles à leurs chemins canoniques absolus et explicitement transmis ; ils ne sont pas supposés présents par magie dans les worktrees. Les index MCP canoniques ne couvrent pas les changements non intégrés des branches.

## Vérification et suite autonome

Le lead examinera les recommandations, ratifiera les contrats conformes dans la délégation puis intégrera les livrables avant les consommateurs. Les nouveaux tests statiques de la branche Envoy seront nommés avant exécution. Pas de Cargo concurrent, démarrage Kind anticipé, revue générale ajoutée ou changement de modèle par override.

Aucun résultat de correction, compilation ou test n'est encore revendiqué. Les ressources runtime de mission restent vides : aucun ID à nettoyer.

## Annexe technique

- Approbation : révision 1, hash `df8486d1c5c18ff89fbb46fa984c02f49259fe6ff307c56b21c182e89809602e`.
- HEAD : `c72ae362b950c63d6736a6818c68babc5a8d6f40` ; SDK : `d60a88b2f2f499768a807941a1b4c80c9a366b3f`.
- Membre déjà admis : pool-1, architecte, tâche identité `task_mvsf2g5q_0003_qqrmnr`, worktree `C:/Users/djden/.local/share/opencode/worktree/186749/ensemble-04d9a2f8-d9lbvu-pool-1`.
- Préflight effectué : état natif d'équipe consulté une fois au démarrage ; Git HEAD/status/pin et configuration Cargo lus dans Context Mode ; présence des numéros ADR contrôlée. Aucun build/test/runtime.
- Sessions : pool-1 `ses_e7c4c8877ffeJutSuR8Wq2Kp6j` ; contrat-signataire `ses_e7c495f4bffepZ9iIO2HzVA7e6` ; contrat-navigateur `ses_e7c47c029ffetuDxXz4nR3y14d` ; noms-serveurs-envoy `ses_e7c474236ffeTbjv0Yrf1JRuYD`. Identités relevées via team_view sans navigation ; worktrees retrouvés par la liste native, sans toucher aux 73 autres worktrees.
- Le contrat navigateur utilise final_response, adapté au profil restrictif sans ajout de permission. Le plan Envoy exige l'approbation du lead avant écriture. Deux spawns ont émis un avertissement de permission effective des outils d'équipe ; cela n'est pas présenté comme un refus concret. Les auteurs ont reçu la consigne de ne modifier aucune permission ni contourner un refus, de vérifier leur répertoire et de rendre au besoin une réponse finale native sourcée. Aucun changement de profil ou de modèle en cours d'admission.
