# Candidat PM reçu — périmètre d'authentification confirmé

## Objectif et décision du superviseur

La mission reste la correction et la qualification de la sécurité de l'authentification. Ensemble est l'outil de coordination, pas un produit à développer dans cette mission. Le dossier figé et la réparation de contexte utilisateur conservent cet objectif ; aucune nouvelle décision utilisateur de changement de chantier n'est nécessaire pour le maintenir.

Le PM a terminé un candidat de quatorze tâches, mais l'a déclaré blocked parce que le brouillon demandait encore de clarifier « finir l'orchestrator ». Le superviseur a remplacé cette instruction obsolète par la confirmation explicite du périmètre. Il confirme aussi que ratification des nouveaux contrats, publication SDK et opérations Kind restent soumises à leurs autorisations distinctes au jalon concerné.

Cette correction lève la contradiction dans le document, pas l'état natif de préparation : aucun candidat blocked n'est converti manuellement en ready et aucune tâche n'est importée par ce travail. La reprise native du même PM est la suite appropriée ; elle n'est pas invoquée automatiquement.

## Revue du résultat complet

Le superviseur a lu le candidat dans la session native du PM, pas seulement sa notification. Il conserve les critères, ownership, dépendances, vérifications, sources et limites utiles. Les trente-cinq chemins de sources cités existent ; aucune dépendance inconnue dans le graphe de quatorze tâches. Ces contrôles documentaires ne prouvent pas que l'implémentation ou ses tests passent.

Les décisions d'architecture sont bien représentées par des livrables et des dépendances ; leur ratification future ne bloque pas la préparation d'un plan. La disponibilité du navigateur et du laboratoire est un livrable de qualification futur, pas une capacité déjà démontrée.

Différences substantielles avec les quinze fiches antérieures du superviseur :

- Le PM propose un contrat ciblé supplémentaire pour les bornes et exceptions du signataire distant.
- Les régressions rouge/correction/refactoring sont regroupées dans les lots de développement plutôt que deux branches de tests distinctes. Les défauts production révélés par la QA restent à leurs propriétaires ; test-engineer ne les corrige pas.
- La qualification des capacités navigateur et de la cible runtime est une étape explicite avant la campagne, sans installation ni réparation de laboratoire implicite.
- Les revues indépendantes suivent la qualification native du snapshot ; elles peuvent lire en parallèle l'une de l'autre.

Le candidat natif devient la proposition de travail privilégiée. Le découpage antérieur n'est pas supprimé, mais son statut historique est explicite. Les durées fournies par le PM sont des estimations non mesurées, pas un devis ni un budget autorisé.

## Aperçu des quatorze lots proposés

1. Contrat de séparation des autorités jusque dans les permissions.
2. Contrat des exigences sécurisées du signataire distant.
3. Alignement du contrat navigateur avec l'entrée du laboratoire.
4. Vérification des noms des serveurs TLS contactés par Envoy.
5. Correction des permissions après intégration et ratification du contrat d'identité.
6. Durcissement du signataire après son contrat.
7. Alignement du laboratoire et des scénarios navigateur après contrat et intégration des changements Envoy.
8. Qualification native Windows du snapshot intégré.
9. Revue indépendante des frontières de sécurité.
10. Revue indépendante des oracles de tests.
11. Qualification des capacités navigateur et de la cible runtime autorisée.
12. Preuve intégrée avec navigateur et vraie passerelle.
13. Vérification infra indépendante, sous bail parent.
14. Réconciliation documentaire selon les preuves, sans promotion implicite de statut ADR.

Les quatre premiers lots n'ont pas de dépendance entre eux. Les autres restent dans le plan tant que leurs entrées ne sont pas intégrées. La reprise doit préserver le graphe complet au dossier mais n'exposer que les branches prêtes à l'import/publication initial, pour respecter la règle locale de non-publication prématurée des consommateurs. Cette contrainte n'est pas remplacée par une simple mention « deferred » dans un texte ni par la fin d'un auteur.

## Correction persistée et suite

Un seul champ du brouillon est corrigé : la dernière instruction long_run confirme le périmètre et les autorisations différées au lieu de demander encore sa clarification. Fournisseur OpenAI, modèles/variantes, seuil, classification et QA proportionate sont inchangés. Le précédent hash de présentation ne couvre donc plus ce brouillon ; une nouvelle présentation par config sera nécessaire avant tout start.

Reprise proposée dans la même session PM :

```text
/ensemble-mission prepare --resume Le superviseur confirme l'objectif exclusif d'authentification du dossier fige. Ensemble reste l'outil de coordination, sans developpement de son plugin. Le brouillon persistant confirme ce perimetre. Conserver ratification des nouveaux contrats, autorisation distincte de publication SDK et autorisation ciblee Kind comme barrieres futures. Conserver le graphe complet de quatorze lots au dossier, mais exposer seulement les quatre racines pretes a l'import initial ; les consommateurs restent au plan jusqu'a integration de leurs entrees selon AGENTS.md. Ne supprimer aucun vrai blocage ni declarer une execution approuvee.
```

Cette reprise est proposée, pas exécutée. Après retour natif admissible et contrôle d'intégration, représenter le brouillon par `/ensemble-mission config`. Le `/ensemble-mission start` reste réservé à l'utilisateur. Aucun worker, test, runtime, publication distante ou code applicatif n'est autorisé par la clarification.

## Annexe de traçabilité

- PM : `ses_e7c953b4dffeSr2aLcciLsCZxQ` ; message candidat complet `msg_18397c324001iciHmkYaUctbKD`, finish=stop.
- Version 2 ; readiness=blocked ; catalogue vérifié ; aucune indisponibilité requise annoncée ; quatorze tâches ; aucune tâche importée selon notification native.
- Question bloquante : `confirmer-objectif-authentification`. Question non bloquante : `autorisations-differees`. Les deux sont traitées ci-dessus dans le rôle de superviseur, sans changer le Statut des ADR.
- Lectures natives session.message.list puis session.message.get, filtrées dans Context Mode. Sources locales vérifiées en existence ; aucune exécution de leurs commandes de test.
- Fichiers mis à jour : model-policy.json, task-plan.md, task-breakdown.md, ce rapport et ledger du lot. Aucune ressource Docker/Kind/WSL créée ou redémarrée.
- Validation effectuée : JSON de politique valide, OpenAI et quatre variantes Sol conservés, QA proportionate, quatorze commandes et douze critères inchangés ; cinq fichiers relus sans caractère de remplacement ni espace final ; `git diff --check` exit 0. Aucun test comportemental exécuté.
