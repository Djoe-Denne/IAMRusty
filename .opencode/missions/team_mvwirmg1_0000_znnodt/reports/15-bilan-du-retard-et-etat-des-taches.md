# 15. Bilan du retard et état réel des tâches

## Résultat attendu et constat

La mission consiste à porter puis exécuter la campagne d'authentification sous k6/Chromium Windows, avec les mêmes garanties que la campagne historique. **Le code est intégré, mais la campagne réelle n'a pas encore été exécutée.** Aucun PASS fonctionnel global n'est acquis. Le déploiement du laboratoire est en cours ; sa fin n'est pas encore attestée.

## Ce qui a raté

Le socle Windows qualifié a été confondu dans l'estimation avec un raccord auth presque prêt. Le socle couvrait les outils, la supervision et le cas navigateur minimal ; il ne couvrait pas le pilotage complet des certificats, du forward fournisseur et des corrélations auth. Le premier livrable du parent était limité à des helpers statiques, sans entrée d'exécution complète. Le lead a dû faire poursuivre le même auteur. Cette insuffisance aurait dû être identifiée et bornée plus tôt.

Deux défauts concrets ont ensuite nécessité des reprises : phase SAN non transmise à l'initialisation de k6 inspect, puis restauration TLS bloquée si IAM n'était plus Ready. Les corrections sont intégrées, l'inspection réelle passe et la revue ferme le défaut de restauration dans le code. Leur traitement était nécessaire, mais les annoncer comme une simple finition d'un ensemble prêt aurait été trompeur.

La remise en place du laboratoire absent a ajouté du travail réel : bail et identités neufs, certificats, images et déploiement. Un échec de génération OpenSSL dû à la conversion des arguments par Git Bash a ajouté un diagnostic et une reprise locale. Ce n'était pas un défaut du produit IAM.

Le lead reconnaît également un coût de coordination excessif : premiers retours d'auteurs réduits à des accusés de réception, relances pour obtenir de vrais livrables, intégration manuelle de deltas non committés et trop de messages de jalon. Ces échanges ne sont pas de la progression fonctionnelle. Les preuves disponibles ne permettent pas d'attribuer honnêtement une durée précise à chaque cause.

## Lots initiaux

1. **Inventaire des garanties et correspondance ancien → nouveau : terminé.** Matrice intégrée. Les 17 IT historiques restent au niveau tests d'intégration, avec réutilisation justifiée par comparaison des sources.
2. **Adaptation minimale du contrat Windows : terminée.** Contrat intégré et ratifié avant le code consommateur.
3. **Implémentation des scénarios : terminée côté code.** Parcours inscription terminée → nouvelle connexion → réassociation réelle, trois refus403 et observations cookies/nonce/headers ; 26 contrôles navigateur. Scénario HTTP certificat : deux contrôles par phase bon/mauvais/restauré. Ils sont intégrés, pas encore exécutés sur la cible.
4. **Raccord parent Windows : terminé côté code et inspection.** Lancement, résumés fermés, corrélations et opérations certificat intégrés. Inspection native PASS, confidentialité/cleanup PASS, aucun navigateur/cible pendant cette inspection. Le tableau natif le gardait encore en cours : le lead vient de corriger cet état administratif après acceptation du code et des correctifs. Cela ne clôt pas son fonctionnement réel.

## Contrôles ajoutés à la publication des lots

5. **Revue indépendante de parité des scénarios : terminée.** Prévue par l'ordre de mission, aucun défaut bloquant ; verdict statique seulement.
6. **Revue sécurité ciblée du raccord : terminée.** Ajoutée en raison du delta réel certificat/processus ; un défaut de restauration trouvé, corrigé puis relu. Pas audit global du framework.

Le tableau comporte donc six tâches publiées, maintenant toutes closes à leur portée. **Il ne couvre pas à lui seul le travail restant de la mission.** Ce manque de lisibilité du suivi est une responsabilité du lead ; les étapes runtime et livraison étaient suivies dans le plan opérateur/ledger, pas encore sous forme de tâches publiées.

## Étapes opératoires et correctifs supplémentaires

- Correction bootstrap `--retain` pour conserver les nœuds en cas d'échec : terminée, test ciblé existant ROUGE puis VERT.
- Correction phase SAN dans inspect : terminée, inspection native désormais PASS.
- Correction restauration avant readiness : terminée et relue statiquement ; restauration réelle non encore prouvée.
- Diagnostic et reprise des certificats Git Bash : terminés. Huit certificats transport et un workload vérifiés, bon nom accepté et mauvais nom refusé sous la même CA.
- Recréation du laboratoire : terminée. Extension explicitement autorisée par la révision2, pas une tâche cachée du plan initial. Trois nœuds neufs, ancrage vérifié.
- Épinglage des dix images existantes : terminé, aucun build applicatif ; image IAM qualifiée retenue, aucun `latest` partagé déplacé.
- Déploiement du profil existant : **en cours**, résultat final attendu.

## Pas encore commencés

- Vérification indépendante de la disponibilité et du trajet réseau après déploiement ; préparation IAM mono-instance préalable.
- Exécution réelle des trois phases certificat et de la restauration, puis du parcours navigateur d'authentification avec ses corrélations.
- Bilan d'acceptation dynamique, publication distante des livrables, nettoyage et archivage final.

La qualification Linux reste différée, hors mission. Aucun nouveau harness de tests, nouvelle fixture ou campagne générale du framework n'a été ajouté.

## Workers et ressources

Sept identités de membres sont visibles historiquement, dont deux sessions d'architecte ; elles ne représentent pas sept lots d'implémentation indépendants. Tous les autres membres sont arrêtés ; l'auteur parent est inactif et conservé pour continuité éventuelle. Les reprises correctives sont revenues au même auteur et au même reviewer, pas à de nouvelles équipes systématiques.

Trois conteneurs de nœuds Kind ont été créés pour remplacer le laboratoire absent. Ce ne sont pas des conteneurs de compilation Windows. Les dix images applicatives existaient et ont été réutilisées ; Docker Desktop était déjà actif et n'a pas été redémarré par cette mission. Les services sont en cours de déploiement dans ces nœuds. Les ressources étrangères sont préservées.

## Preuves techniques

Tableau natif lu au bilan :5completed/1in_progress ; tâche parent corrigée à completed, six tâches publiées closes. Commits : contrat dde0340 ; scénarios0c1e21d ; parent51e74e1 ; correctifsd00ab08. Inspection5dc47c613727497eab36374e9f8b67bd exit0, privacy/cleanupPASS, k6/Chrome0→0. Revue corrective result_mvwnfmh3_0019_efwdpe. Bootstrap exit0, trois IDs complets dans bail neuf, anchor check exit0. Épinglage10images exit0. Déploiement handle sh_192f94c110018CBaCt5zgUc6ey : aucune sortie finale au moment du bilan, aucun succès inféré. Registre .tmp/docker-resources-20261031.log ; ledger .cursor/handoffs/20261029-k6-02-auth.md.
