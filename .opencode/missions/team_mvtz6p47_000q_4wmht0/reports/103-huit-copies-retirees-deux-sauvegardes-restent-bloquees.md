# Huit copies retirées ; deux sauvegardes restent bloquées

## Objectif et résultat

Le cas minimal k6/Chrome Windows reste réussi et publié. Sur les dix anciennes
copies de travail dont le retrait a été autorisé, **huit sont maintenant retirées**,
après sauvegarde et restauration vérifiées. Le contrôle Git, le contrôle des dossiers
sur disque et l'inventaire natif OpenCode concordent : seules deux copies subsistent.
Les sauvegardes restent disponibles hors des dossiers supprimés.

Les références de sauvegarde Git sont inchangées. Le checkout principal, ses
modifications étrangères, son sous-module et les outils installés sont conservés.
Aucun runtime Docker, Kind ou WSL n'a été démarré ; aucune donnée WSL n'a été touchée.

## Pourquoi deux copies restent présentes

Les copies des anciens auteurs du conducteur WSL et du premier raccord Windows
contiennent encore des sources non committées. Des lectures externes avaient été
refusées auparavant. L'autorisation de nettoyage exige une sauvegarde vérifiée :
**ces deux copies ne sont donc pas supprimées**.

La demande de livraison native adressée à l'ancien auteur WSL est seulement mise
en file pour un membre arrêté, pas exécutée. Il n'y a aucun auteur actif à attendre.
Le lead n'a pas recréé de membre, ouvert de nouvelle session ou modifié des droits
pour contourner ce résultat. Les journaux disponibles de diagnostic de permissions
ne précisent pas la cause de l'ancien refus ; leur absence de corrélation ne le lève pas.

La récupération restante vise exactement **32 fichiers source publics** : 22 dans
la copie WSL et 10 dans la copie Windows. Les noms sont inventoriés, mais leur
contenu courant n'a pas été relu. Les logs, profils, configurations de campagne,
baux et binaires sont exclus ; le journal privé de fixtures Windows reste exclu.

L'archivage normal de l'équipe n'est pas lancé tant que ces modifications utiles
ne sont pas récupérées. Sa précondition de conservation du travail n'est pas remplie.
Le ticket Linux reste différé et non publié ; il n'est pas exécuté pour vider le tableau.
La mission n'est donc **pas administrativement clôturée**, sans incidence sur le
résultat Windows déjà utilisable.

## Options pour cette dernière récupération

- **Autoriser une copie native ciblée des sources** : autoriser explicitement
  une nouvelle opération de lecture/copie, via l'outil natif de fichiers ou de shell,
  limitée à ces 32 sources et à une sauvegarde locale hors des deux arbres.
  Aucun réglage global, permission MCP ou protection Windows ne sera modifié.
  Les fichiers restaurés seront comparés avant retrait ; un nouveau refus sera
  respecté et les dossiers resteront présents. C'est la recommandation pour achever
  le nettoyage sans perdre de travail. Le temps nécessaire n'est pas estimé.
- **Conserver les deux copies** : aucune lecture ou suppression supplémentaire.
  Les sauvegardes des huit copies retirées et le résultat Windows restent acquis ;
  le disque reste occupé par les deux dossiers et l'archivage demeure en attente.

Une archive des mêmes sources fournie directement par l'utilisateur permettrait
également une reprise, après contrôle des empreintes et de son exhaustivité.

## Annexe technique

Retraits natifs `opencode.worktree_remove`, tous avec résultat `removed:true`,
forçage limité à l'autorisation explicite après sauvegarde : pool-3, 4, 7, 9, 10,
12, 22 et 23. Copies conservées : pool-13 et pool-20, aux deux chemins exacts du
rapport100. Aucun retrait ou purge de référence préservée ou d'archive d'équipe.

Sauvegarde : `.tmp/k6-closeout-backup-20261031/`, avec sous-dossiers `working`,
`base`, `index` et `verified-restoration`. Manifeste SHA256 :
`8a14e2d0155e7b7b461fd7b1dc4e827738e1dc1da3fc9e22ecbc115e01aa8079`.
47 fichiers working, leurs bases/index disponibles et quatre patchs publics sont
conservés. Les anciennes propositions ont une disposition explicite au rapport102,
sans les rétablir dans les contrats ou sources actuels.

Preuves privées : `.tmp/k6-cleanup-metadata-20261031.json`,
`.tmp/k6-cleanup-disposition-20261031.json` et `.tmp/k6-cleanup-after-20261031.json`.
Le dernier inventaire confirme huit absences, deux présences, références préservées
inchangées et HEAD du sous-module inchangé. Aucun test fonctionnel n'est rejoué
pour ce nettoyage documentaire ; les 34 tests Windows et le vrai cas navigateur
du rapport99 restent à leur portée. HTTP, WSL et Linux restent INCONCLUSIVE.
