# Préparation prête — quatre tâches importées, exécution non approuvée

## Résultat et portée

La mission de sécurité de l'authentification dispose maintenant d'une préparation native ready, sans question ouverte. Le superviseur a relu le document complet et contrôlé le tableau : seules les trois décisions de contrat et la correction TLS initiale sont importées, toutes pending. Les dix consommateurs restent dans l'annexe documentaire du PM, sans publication prématurée.

Le [plan natif retenu](../prepared-plan.md) remplace le découpage provisoire du superviseur pour la suite. Aucun changement d'objectif ou élargissement de QA n'est ajouté. L'import n'est pas une approbation d'exécution ; aucun worker, Cargo, test ou runtime n'a été lancé.

## Les quatre départs proposés

- Séparer les autorités jusqu'au contrôle des permissions : contrat par architecte, aucune production.
- Fixer les exigences sécurisées du signataire distant : bornes, exceptions et activation, aucun nouveau fournisseur.
- Aligner le contrat navigateur : entrée, confiance, cookies et connecteur, sans laboratoire démarré pour décider.
- Vérifier les noms des serveurs joints par Envoy : correction statique compatible, preuves réseau différées à la campagne autorisée.

Ces lots peuvent avancer indépendamment après approbation, en worktrees séparés. Le lead réserve les numéros ADR et sérialise leur index. Les contrats restent Proposed jusqu'à ratification. Les consommateurs ne sont publiés qu'après intégration vérifiée de leurs entrées ; une fin d'auteur ne suffit pas.

## Politique à présenter

OpenAI uniquement, Sol avec variantes low/medium/high/xhigh ; seuil de départ 50, classification désactivée. Long_run proposé, pas encore actif. Sécurité prioritaire, architecture respectée, code propre et réutilisation d'Ensemble. QA proportionate avec quatorze commandes de référence et douze critères, choisis selon les risques et preuves valides.

Proportionate reste la recommandation et le choix actuel : vérifications ciblées, revues indépendantes utiles, sans pipeline général. Light réduirait les contrôles des auteurs sans supprimer les obligations du dépôt ; reinforced ajouterait un plan explicite plus étendu. Aucune nouvelle décision QA n'est nécessaire en l'absence de correction utilisateur.

Le brouillon corrigé a désormais été représenté par la commande native config, acceptée avec exit 0 puis proposition native reçue. L'exécution reste explicitement paused/not approved ; le start appartient à l'utilisateur. Contrats nouveaux, publication SDK et laboratoire Kind conservent leurs barrières propres ; aucun de ces accords n'est remplacé par start.

## Contrôles et limites

- Document natif version 2 : readiness ready, reasons et missingInformation vides ; catalogue vérifié, aucune indisponibilité requise, aucune question.
- Quatre tâches sur le tableau : toutes pending, profils et contextes fresh cohérents, isolation présente, completion détenue par le lead. Le mot authorized décrit la décision de planification par tâche, pas l'approbation de la mission.
- Graphe complet : quatorze lots, dont dix décrits dans l'annexe et non importés. Aucun résultat blocked transformé manuellement en ready.
- Les preuves initiales restent statiques et les durées PM indicatives ; aucun nouveau résultat comportemental ou budget garanti n'est revendiqué.

## Annexe de traçabilité

- PM : `ses_e7c953b4dffeSr2aLcciLsCZxQ` ; candidat ready `msg_183a6f62b001DcW1GQl6qNN467`, finish=stop.
- Identités/permissions (`arch-identites`) : `task_mvsf2g5q_0003_qqrmnr`.
- Signataire sécurisé (`arch-signataire`) : `task_mvsf2g5q_0004_ft9ns7`.
- Contrat navigateur (`arch-navigateur`) : `task_mvsf2g5q_0005_ximvvi`.
- Noms serveur Envoy (`corriger-envoy`) : `task_mvsf2g5q_0006_29odow`.
- Preuves de revue : notification native d'import, lecture native des messages PM, outil team_tasks_list consulté une fois pour contrôler l'import. Pas de polling d'attente.
- Aucune ressource Docker/Kind/WSL créée ou redémarrée ; aucun ID à nettoyer.
- Présentation config reçue : hash `d3f86aaa2a8eb6aacf76b7402e83a005240980fac2d02aa230e12eb306bdcfc4`. Aucun start invoqué, aucun changement de politique après cette présentation.
- Vérification documentaire effectuée : plan retenu avec quatorze fiches, cinq fichiers relus sans caractère de remplacement/espace final, politique JSON valide (OpenAI, quatre variantes Sol, proportionate, quatorze commandes/douze critères), `git diff --check` exit 0. Pas de nouveau résultat comportemental.
