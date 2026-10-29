# Diagnostic navigateur publié — un essai CI discriminant en cours

## Objectif et changement

Le lanceur peut maintenant ajouter un code fermé décrivant certains messages de démarrage du navigateur. Aucun texte natif, chemin, URL ou environnement n'est publié. Les verdicts, exigences de sandbox, contrôles de confidentialité et refus d'artefacts restent inchangés.

Cette amélioration ne prouve aucune cause ni réussite. La recherche officielle montre aussi une limite : k6 peut écarter certaines lignes fatales Chromium avant que notre parent ne les voie. Une cause absente des flux restera inconnue ; AppArmor n'est pas désigné par supposition.

## Vérifications et prochaine preuve

Le lead a vérifié les cinq fichiers identiques au candidat et exécuté 56 tests Windows : 49 exercés, 7 spécifiques à Linux ignorés, exit0. Le commit et son vrai hook sont réussis. Le contenu est publié sur master, référence distante vérifiée.

Une seule nouvelle exécution native, déclenchée par ce push normal, doit fournir le diagnostic manquant et exercer les tests Linux ajoutés. Le lead attend son résultat sans relancer la même expérience en boucle. La qualification HTTP reste fermée et le navigateur ne devient pas vert par diagnostic.

## Annexe technique

- Commit publié `4e82a365077c505bb6681bbbb3018f3869f995b7` ; cinq fichiers sous ops/tests/k6, aucune modification du workflow, pins ou configuration système.
- [Essai CI discriminant](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40965632254), événement push ; résultat encore attendu.
- Watch natif asynchrone, log privé `.tmp/k6-ci-watch-40965632254.log`, pas de polling manuel.
- Sources retenues et limite du parseur k6 documentées dans README/IMPLEMENTATION ; la classification est lexicale, pas une observation du noyau.
- Sauvegarde récupérable des 14 fichiers finaux et de l'index auteur : `.tmp/k6-final-source-4e82a36-0ca61fbbf2db4b4e8da1a70051cd1ec8`, SHA256 vérifiés, égalité au canon publié. Aucun arbre, index ou branche supprimé/modifié par cette sauvegarde.
