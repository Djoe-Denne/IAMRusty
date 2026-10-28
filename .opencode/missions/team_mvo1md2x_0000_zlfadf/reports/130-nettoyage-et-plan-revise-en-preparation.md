# 130. Nettoyer les diagnostics avant de reprendre la preuve

## Objectif et étape actuelle

Livrer la publication durable des notifications, puis leur reprise jusqu'au droit d'accès HTTP. Après la recherche web, l'utilisateur demande d'appliquer l'approche minimale, de retirer les changements contraires et de réviser le plan avant de reconnecter l'exécution.

Le PM prépare ce plan dans la mission existante. Le lead audite en parallèle le diff réellement intégré. Aucun test, build ou démarrage de runtime n'a été lancé à cette étape.

## Constats vérifiés

- Les changements SDK depuis le pin portent sur HTTP et ses dépendances, pas sur le dispatcher outbox. Aucun raccourcissement de lease, déverrouillage anticipé ou drain nouveau n'est établi par cet audit.
- Le diff non committé comporte deux documents, la correction utile du contrôle temporel au premier état FAILED et 764 lignes de témoins composant. Le checkout SDK local reste distinct du pin livré.
- Le premier témoin composant observe la conservation du verrou avant expiration ; le second prépare la preuve avant et après expiration. Certains helpers sont communs : supprimer le premier bloc sans examiner leurs usages casserait le second.
- Les documents accumulent une chronologie désormais redondante avec les rapports et le ledger. Ils peuvent être ramenés à une photographie actuelle concise, sans réécrire les critères normatifs.

## Nettoyage envisagé, pas encore livré

Conserver les corrections transactionnelles, NoOp, HTTP et le contrôle temporel firstFAILED. Examiner le retrait du témoin autonome limité à la fenêtre avant expiration si le témoin complet conserve ses garanties pertinentes ; garder son source et ses résultats archivés. Ne pas supprimer des tests uniquement à cause de leur taille. Simplifier la photographie documentaire et retirer les prochaines étapes devenues obsolètes.

Le PM doit expliciter tâches ajoutées, retirées et conservées, dépendances, propriétaires et critères de vérification. Le nettoyage sera intégré et contrôlé avant de reconnecter la preuve ciblée après expiration. Aucune nouvelle architecture, campagne globale, publication SDK ou mise à jour finale du pin n'est couverte ici.

## Limites

La preuve après expiration reste absente : la tentative précédente avait échoué sur un identifiant de campagne invalide, avant initialisation des fixtures. La cause historique du timeout vertical reste inconnue. Les sources archivées, rouges et revues restent conservés ; aucun succès global n'est annoncé.

## Traçabilité technique

- Recommandation et sources web : [rapport de recherche](129-recherche-web-patterns-outbox-et-arret.md).
- Source examinée : worktree `AIForAll/.worktrees/ti`, AI HEAD `1f2dd30`, SDK local `d60a88b` propre.
- Lectures : `git diff` ciblé des deux documents et des deux tests ; inventaire des chemins SDK depuis `129dcc1` ; blocs composant lignes 401–440 et 717–814.
- Planification native : `task_mvqtl9kf_000n_avf04w`, propriétaire PM `pool-21`, lecture seule, contexte fresh. Aucun consommateur de nettoyage publié avant son résultat.
- Ledger canonique actualisé : `.cursor/handoffs/20261025-telegraph-notification-outbox.md`.

Localisation : tests Telegraph et photographie documentaire. Impact : audit et préparation du nettoyage, pas correction de production. Limites : aucun nouveau résultat comportemental ni inventaire runtime déduit des campagnes historiques.
