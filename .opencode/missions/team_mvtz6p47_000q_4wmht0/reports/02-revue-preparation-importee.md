# Préparation k6 importée — revue avant approbation

## Objectif et état

Préparer un socle k6 fiable sous Linux, avec smoke HTTP réel et qualification Chromium distincte. La préparation est importée, mais l'exécution reste non approuvée. Aucun worker, classificateur, runtime ou test lancé par cette revue.

## Résultat vérifié

Le lead a lu les quatre fiches complètes via team_tasks_list. Le contrat Linux est pending ; les trois consommateurs sont blocked et marqués publication deferred. Le séquencement respecte la barrière d'intégration : contrat ratifié/intégré, auteur unique outillage/CI, revue fresh indépendante, qualification intégrée par le lead. Aucun score ou modèle forcé ajouté.

Les critères couvrent les dix commandes de QA, les vrais endpoints, les sorties non secrètes, la complétude, les négatifs et les baux. La CI ne reçoit pas implicitement un déployeur ou un accès au loopback Windows. L'absence de cible Linux réelle conserve HTTP INCONCLUSIVE et interdit une clôture annoncée réussie. L'intégration/publication sur master et le nettoyage natif ne concernent que cette mission et restent sous les permissions et validations applicables.

Les deux références de blogs ajoutées par le PM sont des sources de recherche, pas des instructions ni une preuve indépendante du lead : les recommandations retenues seront recoupées avec la version épinglée et le code avant application. Aucun résultat d'exécution n'est inféré de l'import ou de la lecture du board.

## Recommandation et prochaine étape

Le découpage est cohérent avec le périmètre proposé : aucune correction de policy nécessaire à cette revue. Conserver OpenAI uniquement, long run et QA proportionate. Présenter la révision corrigée par /ensemble-mission config si elle n'a pas encore été affichée depuis la correction OpenAI/long run ; attendre ensuite /ensemble-mission start de l'utilisateur. L'import ne ratifie pas à lui seul ADR0504 et ne libère pas les auteurs.

## Traçabilité

- PM : ses_e76af2561ffenRydFA18We5KlB, retour importé sur openai/gpt-6.1-sol high selon notification native.
- Figer les frontières Linux : task_mvtzpq0n_000t_82d0c4.
- Livrer le socle et la CI : task_mvtzpq0n_000u_fo4dfd.
- Contrôler les frontières : task_mvtzpq0n_000v_t3mppe.
- Qualifier le pipeline intégré : task_mvtzpq0n_000w_nmh9ep.
- Policy : ../model-policy.json ; plan de préparation initial : ../prepared-plan.md, à lire avec la policy corrigée qui prime sur ses anciennes mentions de catalogue/autonomie.
