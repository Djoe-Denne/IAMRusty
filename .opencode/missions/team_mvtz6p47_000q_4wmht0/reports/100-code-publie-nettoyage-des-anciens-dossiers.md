# Code publié ; nettoyage des anciens dossiers à décider

## Objectif atteint et dernière étape

Le cas minimal k6/Chrome Windows est réussi : about:blank, calcul 42, version
épinglée vérifiée, nettoyage et confidentialité PASS. Les **34 tests Windows finaux
réussissent sans skip**. Le code et les preuves sont publiés sur la branche distante
master ; sa référence a été vérifiée après le push. Le résultat est utilisable sans
attendre le nettoyage des anciennes copies de travail.

Il reste **dix dossiers temporaires de cette mission**, utilisés par les agents.
Certains contiennent encore des modifications non committées. Leurs commits de
base sont tous déjà dans master, mais cela ne suffit pas à sauvegarder ces fichiers.
Les supprimer de force n'est pas autorisé implicitement et une sauvegarde complète
vérifiée de ces modifications reste à établir. Aucun dossier n'a été supprimé.

**Recommandation :** sauvegarder les changements utiles, vérifier leur récupération,
puis retirer les seuls dossiers temporaires de cette mission. Ne retirer aucun arbre
dont la sauvegarde est incomplète, dont la disposition n'est pas vérifiée ou dont les
données doivent encore être conservées. Les anciennes difficultés de lecture des
outils ne constituent pas une permission de contourner leurs refus ou d'effacer.

## Choix et conséquences

- **Sauvegarder puis nettoyer** : autoriser le retrait forcé des dossiers modifiés
  seulement après sauvegarde vérifiée de leurs changements utiles et contrôle de leur
  disposition. Les copies de travail et leurs artefacts jetables seront supprimés ;
  le code publié et les sauvegardes restent. Durée et espace nécessaires non estimés.
- **Conserver les dix dossiers** : aucune suppression. Le test réussi et les sources
  publiées restent disponibles ; les dossiers occupent encore le disque et la clôture
  administrative reste en attente.

Dans les deux cas : préserver le checkout principal, les modifications étrangères,
le sous-module actuel, les outils k6/Chrome, les données WSL et son VHDX sur I:.
Aucune purge des archives d'équipe ni des références de sauvegarde n'est demandée.
Pas de nouveau runtime, test ou changement fonctionnel pour ce choix de nettoyage.

## Vérifications effectuées depuis le rapport précédent

- Push normal, sans forçage, avec hooks conservés : code retour 0.
- `git ls-remote` confirme master au commit fonctionnel
  `201ec05b9b2753a4623cb0c8842a2a6a012e172e` après publication.
- Workflow k6 Linux SUSPENDED, deux conditions littérales fausses ; Git LFS disponible
  et son hook de pre-push conservé. Aucun dispatch ou qualification Linux déduit.
- Tous les 23 agents sont arrêtés. Le dernier auteur a été accepté après intégration,
  essai réel PASS et suite finale. La fusion native ne trouve aucun commit de sa
  branche : ses fichiers non committés restent préservés, pas perdus ou supprimés.
- Inventaire Git et dossiers : dix arbres de mission présents et enregistrés ; tous
  leurs HEAD sont ancêtres de master. Le premier ancien arbre est déjà absent.
- Les seules modifications suivies étrangères observées sur le checkout principal
  concernent l'état Obsidian et le sous-module rustycog ; elles sont laissées intactes.
- Le lot historique Linux est retiré de ce mandat au profit de la révision Windows.
  Sa disposition ne signifie pas qu'une qualification Linux ait été exécutée.
  Son ticket reste différé : l'outil refuse sa clôture sans publication préalable.
  Il n'a pas été publié, afin de ne pas déclencher un travail Linux hors mandat.

Les limites fonctionnelles du [rapport de preuve Windows](99-cas-minimal-windows-verifie.md)
restent : HTTP/selftests réels INCONCLUSIVE, WSL suspendu et Linux INCONCLUSIVE,
aucune attestation du sandbox. Aucun processus k6/Chrome restant après la campagne ;
les 11 fixtures des tests finaux sont observées terminées et leurs dossiers retirés.

## Périmètre technique exact du retrait proposé

Sous `C:/Users/djden/.local/share/opencode/worktree/186749/`, les seuls dossiers visés
sont les suivants ; aucun autre nom ou motif global n'est autorisé :

- `ensemble-04d9a2f8-4wmht0-pool-3`
- `ensemble-04d9a2f8-4wmht0-pool-4`
- `ensemble-04d9a2f8-4wmht0-pool-7`
- `ensemble-04d9a2f8-4wmht0-pool-9`
- `ensemble-04d9a2f8-4wmht0-pool-10`
- `ensemble-04d9a2f8-4wmht0-pool-12`
- `ensemble-04d9a2f8-4wmht0-pool-13`
- `ensemble-04d9a2f8-4wmht0-pool-20`
- `ensemble-04d9a2f8-4wmht0-pool-22`
- `ensemble-04d9a2f8-4wmht0-pool-23`

Inventaire privé : `.tmp/k6-closeout-inventory-20261031.json`. Les diagnostics privés,
configurations de campagne, profils, caches et binaires ne sont pas des livrables
à publier. Ce rapport ne prétend ni nettoyage achevé ni équipe archivée.
