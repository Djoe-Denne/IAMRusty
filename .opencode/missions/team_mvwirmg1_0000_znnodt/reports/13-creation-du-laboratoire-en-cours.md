# 13. Création du laboratoire autorisé

La campagne doit maintenant disposer d'une cible réelle avant ses appels d'authentification. Les corrections sont intégrées, l'inspection native passe et la revue indépendante ferme le défaut de restauration dans le code.

Les certificats ont été générés dans un répertoire neuf : huit certificats de transport et un de workload vérifiés ; le bon nom IAM est accepté et le mauvais nom signé par la même autorité est refusé. Les sorties partielles initiales restent conservées.

La comparaison exacte des sources IAM/connecteur, crates et dépendances avec la preuve historique ne montre aucun changement. Les 17 tests conservés sont réutilisés ; aucun test conditionnel 0412 n'a été relancé.

## Opération lancée

Le bootstrap existant est lancé pour le seul `aiforall-local-full`, avec un bail neuf, un kubeconfig privé et la topologie autorisée d'un nœud de contrôle et deux workers. Le préflight a confirmé le contexte Docker attendu et l'absence de ce laboratoire. Les ressources étrangères ne sont pas adoptées.

Les commandes exactes et inventaires sont conservés dans la session privée. Le registre des ressources est ouvert avant la création opaque ; les IDs seront rapprochés du nouveau bail. En cas d'échec, le script tente l'arrêt gracieux des seuls IDs créés et tracés, sans suppression ni nettoyage global.

## Limites et suite

Le résultat de création est encore attendu. Aucun déploiement ni appel auth n'est revendiqué. Après création : images immuables, déploiement existant, préparation IAM mono-instance puis vérification indépendante de l'identité, de la disponibilité et du trajet réseau avant les scénarios. La restauration réelle et la parité dynamique restent non qualifiées.

Annexe : bootstrap `.tmp/20261031-bootstrap-auth-local-full.ps1`, exécution arrière-plan `sh_192f6f3a0001bGetqfO4SaTlCY` ; registre `.tmp/docker-resources-20261031.log`. HEAD exécuté d00ab08. Aucun nouveau GO de jalon demandé.
