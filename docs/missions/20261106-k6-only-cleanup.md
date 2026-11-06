# Nettoyage : k6 seul pour les E2E

## Décision utilisateur du 6 novembre 2026

Après arrêt explicite du run Python, l'utilisateur demande de retirer les anciens
lanceurs E2E hors k6 et de nettoyer toutes les branches locales et copies de
travail secondaires, en rapatriant les livrables utiles sur la branche principale.
Ici cette branche est `master`, conservée sans renommage. Cette décision remplace
l'obligation de conserver les anciens pilotes dans le contrat k6 du 31 octobre ;
elle ne modifie pas les oracles métier ni ne qualifie leur portage.

## Code retiré et code conservé

Retrait de `ops/tests/local-full-acceptance.py`, `local_full_e2e.py` et leurs deux
suites unitaires propres ; retrait de `ops/tests/oauth-browser-local-full.js` et
de son seul test source ; retrait des quatre scripts `ops/scripts/mesh-authn-e2e*`
et `mesh-authn-kind-e2e*`. Les commandes Python de smoke et route-matrix qui
appartenaient au même ancien pilote ne sont plus des interfaces disponibles.

Conservation des scénarios, parents, adaptateurs et tests de `ops/tests/k6/`,
notamment ses auxiliaires Python. Conservation des tests unitaires/IT applicatifs,
du runtime, des fixtures utiles, des certificats, secrets et données. Le helper
de cible et ses tests de tunnel restent conservés : supprimer le pilote E2E ne
donne pas permission de retirer une protection ou un outil partagé.

Les anciens contrats, matrices et rapports restent des archives de connaissance,
pas des commandes exécutables ni des preuves valables pour une nouvelle campagne.
Les points d'entrée actuels sont décrits dans `ops/tests/k6/AUTH-WINDOWS-PARENT.md`.

## Limites explicites de couverture

Le code k6 OAuth livré est encore câblé sur GitLab et le fournisseur synthétique.
Il n'est pas raccordé au profil GitHub seul par ce nettoyage. Aucun test k6 réel
n'est lancé ici. Les 39 obligations mesh et six Apparatus restent un inventaire
historique à rapprocher du circuit retenu : leur suppression du pilote Python
ne vaut pas portage, réduction silencieuse du décompte ou PASS. Les six Apparatus
étaient déjà explicitement non raccordés dans ce pilote.

Le run original a été interrompu à la demande de l'utilisateur. Son accord, sa
consommation, son reçu et son verrou sont préservés. Il n'a pas de bilan final
ni de terminal RETAINED durable prouvé ; aucun réarmement automatique n'est admis.

## Préservation et nettoyage Git

Avant retrait : sauvegarde externe vérifiée des 140 branches locales, des patchs
et des fichiers non suivis de 34 arbres, sans refus de sauvegarde. Le bundle Git
a été vérifié, avec couverture de chaque référence. Aucun contenu de credentials
ou de certificat d'exécution n'est exporté dans cette documentation.

Le tri a identifié 12 commits distincts non ancêtres de master, sur 19 références
historiques : ils doivent être rapprochés du code intégré plutôt que fusionnés
aveuglément. Les ajouts non committés de l'ancienne activation additive sont
abandonnés par la décision GitHub seul ; ne pas les réintroduire.

La purge native des archives Ensemble est actuellement bloquée par des anciennes
associations de branches hors de leur espace de conservation. Pas de contournement
par suppression manuelle de références protégées. Nettoyage Git non terminé tant
que ce blocage et la disposition des copies de travail ne sont pas résolus.

Progression après publication du retrait : les 33 copies de travail secondaires
ont été retirées par l'outil natif après sauvegarde ; Git ne recense plus que le
checkout principal sur master. L'équipe courante a été archivée par son cleanup
natif. Ce cleanup n'a introduit aucun changement de code dans le checkout, ce
qui a été vérifié avec le diff. Le nombre de branches locales est passé de 140
à 82 ; les associations anciennes bloquantes restent à résoudre. Le retrait des
anciens moteurs est publié en `6f2d7632df5aa6798d3643c9c4d7f1be22a72870`,
`origin/master` relu identique. Cela ne vaut pas clôture du nettoyage global.

Le changement Obsidian préexistant a été sauvegardé dans un fichier externe exact,
empreinte relue, puis le seul fichier suivi a été restauré sur choix explicite
de l'utilisateur. Aucun autre travail utilisateur n'a été restauré ou écrasé.
Les 51 fichiers de mission ont été archivés et vérifiés hors du dépôt ; la purge
native de cette seule équipe a ensuite été confirmée par l'utilisateur et exécutée
(34 membres, 28 tâches, 113 messages). Les autres archives ne sont pas purgées.

## Blocage rencontré : compatibilité des archives natives (résolu ensuite)

Le diagnostic du plugin natif localise le refus dans `validatePurgeResources`
et la reconnaissance des anciens noms dans `staleEnsembleBranchNames`. Certaines
branches anciennes incluent le nom du projet et un suffixe d'équipe, format que
cette liste ne reconnaît pas. Il reste 82 branches locales, pas une seule.

Finir par l'outil natif nécessite de réparer cette compatibilité, sans désactiver
les contrôles de propriété, de namespace ou d'équipe active. Cela implique le
plugin OpenCode installé hors de ce dépôt ; aucune modification de ce plugin ou
de sa base de données n'est effectuée sans mandat explicite. Supprimer les refs
ou modifier la base pour éviter le refus n'est pas le repli retenu.

Le projet n'est donc pas annoncé entièrement nettoyé : code et copies secondaires
sont traités, mais les branches historiques et leur disposition native restent
un travail ouvert. La sauvegarde vérifiée est conservée sous
`C:/Users/djden/AppData/Local/Temp/opencode/aiforall-cleanup-20261106-sPA514/`.
Les 72 fichiers temporaires du run et les 12 fichiers d'inventaire/outillage du
nettoyage sont également archivés avec vérification d'empreinte, puis leurs deux
dossiers précis retirés du dépôt. Les registres Docker, autres missions et
autorités privées du run ne sont pas effacés.

### Décision de reprise requise

Option recommandée : autoriser une correction ciblée de compatibilité des anciens
noms dans le plugin Ensemble installé, avec tests des noms reconnus et refusés,
sans modification de base de données ni affaiblissement des contrôles. Risque :
le plugin est global et utilisé par d'autres projets ; le changement doit rester
borné et vérifié. Ensuite reprendre le tri des branches et la purge native avec
les confirmations exactes qu'elle exige. Aucun délai ou résultat garanti.

Alternative : ne pas modifier ce plugin. Le code nettoyé, le checkout principal
et les sauvegardes restent conservés, mais les 82 branches locales subsistent et
l'objectif de nettoyage intégral n'est pas atteint. Aucune nouvelle autorisation
de supprimer les données du runtime n'est demandée ou déduite de ces options.

## Reprise autorisée : compatibilité native réparée

L'utilisateur a autorisé la correction ciblée du plugin global. Une seule forme
exacte de nom historique, corrélée au projet, au nom et à l'identifiant d'équipe
et au membre, est ajoutée à `staleEnsembleBranchNames` dans le bundle installé
et sa source TypeScript de déploiement. Aucune base de données modifiée à la
main, aucun contrôle de propriétaire, namespace ou équipe active supprimé.

Une régression sur les fonctions réellement extraites du bundle installé est
rouge avant correction, puis 11 tests réussissent : ancien nom exact reconnu,
trois formats précédents conservés, projet/équipe/identité/membre étrangers,
suffixe supplémentaire, master et namespace préservé étranger refusés ; usages
par équipe active toujours bloqués. Syntaxe du bundle vérifiée. Sauvegarde du
bundle, de la source et des métadonnées dans `ensemble-plugin-before/` de la
sauvegarde externe. Ce n'est pas l'exécution de la suite complète du plugin.

L'appel natif suivant reconnaît désormais ces branches et retourne un aperçu
valide, sans suppression : sept archives de ce projet,104membres,81tâches,
850messages,11branches préservées,15anciennesbranches et7références périmées de
copies/workspaces Git. Ce dernier décompte n'est pas un inventaire Docker/K8s.
La purge définitive exige la confirmation native exacte `c06b57b8` ; la nouvelle
correction ne dispense pas de cette approbation. Le code publié et le bundle de
sauvegarde restent conservés. Refuser conserve ces archives et leurs références.

## Disposition des branches historiques sans archive native restante

La purge des sept archives a été approuvée avec son jeton exact et exécutée.
L'outil natif confirme ensuite : aucune équipe archivée restante pour ce projet.
Les références encore présentes sont donc des branches locales orphelines, à
retirer par Git après vérification du bundle, pas une façon d'éviter la purge.

Les anciens commits non ancêtres ne sont pas fusionnés aveuglément :

- `f2aa9e2` est intégré avec les corrections finales par `65d2219` ; `2fe6e60`
  par `aa400f3` ; `bc6760a` par `deac598` ; `91a8172` par `36357f6` ; `a456935`
  par `b291eee`. Le code courant conserve les variantes finales du cycle HTTP,
  pas leurs implémentations intermédiaires de tâches/oneshot.
- `ca3e43d` est conservé dans les sources transactionnelles/outbox actuelles ;
  les lignes ajoutées existent, le seul import différent a été regroupé.
- `c9566a3` est rapproché de `e8987ed`, avec les assertions HTTP finales ;
  `d36eac6` de `13d22ab` et `1f2dd30` (test et compatibilité port partagé).
- Les ajouts du plan QA `4c91bc3` et du correctif de racine `1e91353` sont présents
  dans les fichiers canoniques. Ceux de `161487f` sont conservés avec les
  corrections ultérieures des preuves d'endpoint, pas leurs anciennes URL fixes.
- Le contrat `3a685da` est présent avec sa décision et ses corrections finales ;
  ne pas le rétrograder au statut Proposed de cette branche historique.

La comparaison des ajouts, les sources actuelles et le journal d'intégration
ont été rapprochés. Les patchs non committés restants étaient soit des versions
déjà intégrées/corrigées, soit les anciens pilotes et activation additive
explicitement abandonnés. Aucun code Rust, SDK ou ancien runtime n'est restauré
par une fusion administrative. Toutes les références originales restent aussi
récupérables dans le bundle vérifié hors du dépôt.

## État final du nettoyage

- Un seul checkout AIForAll, sur une seule branche locale : `master`. Les 33
  copies secondaires et 139 branches locales supplémentaires sont retirées.
- Huit archives d'équipes supprimées au total, avec les deux confirmations
  natives exactes ; aucune équipe archivée restante pour ce projet.
- Sous-module RustyCog propre au même pin `93b890ff7b821e9cb2e7d0aeb81957d098778a2f`,
  sans branche locale ni copie secondaire supplémentaire. Ses quatre anciennes
  branches sont sauvegardées dans un bundle séparé vérifié.
- Une correction SQS SDK réellement non intégrée a été rapatriée sur master
  comme [patch de travail non appliqué](../recovered-work/README.md). Ne pas
  prétendre qu'elle fonctionne aujourd'hui ni modifier le SDK sans validation.
  Le code de lecture des tuples OpenFGA était déjà intégré dans le SDK épinglé.
- La correction appliquée au plugin global est conservée comme petit patch
  source dans le même dossier, avec provenance et niveau de tests explicites.
- Données, certificats, secrets et environnement GitHub/IAM conservés comme
  demandé ; aucun Docker stop/delete/prune, reset Kubernetes ou nettoyage métier.
  Le runtime est réservé à la reprise k6 ; aucun nouveau test réel n'est lancé.
- Preuves et fichiers temporaires de cette mission archivés hors dépôt avec
  empreintes vérifiées ; ancienne modification Obsidian également récupérable.
- Validations : 11 tests d'alignement source, 11 régressions du bundle Ensemble,
  syntaxe du bundle et contrôles de diff. Aucun PASS k6/GitHub ou portage des
  39 obligations mesh/six Apparatus déduit du nettoyage.

Les paragraphes précédents décrivant les étapes bloquées constituent la
chronologie, pas des blocages encore ouverts du nettoyage Git.

## Vérification du retrait des anciens moteurs

Les 11 tests restants de `test_oauth_browser_alignment_source.py` passent après
retrait du seul test consacré au navigateur autonome supprimé. `git diff --check`
réussit. Aucun build Rust, E2E ou démarrage de runtime lancé. Le retrait ne constitue
pas un nouveau PASS des anciennes suites maintenant supprimées.
