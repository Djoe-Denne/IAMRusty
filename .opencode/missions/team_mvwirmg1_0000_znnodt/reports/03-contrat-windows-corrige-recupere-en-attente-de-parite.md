# 3. Contrat Windows corrigé, rapprochement de parité en cours

## Objectif et étape

Porter les scénarios d'authentification définis vers k6 Windows, sans refaire le framework. Le contrat documentaire minimal a été relu puis corrigé par son auteur. Il est présent dans le checkout principal mais reste Proposed : la correspondance complète avec les garanties historiques précède sa ratification et le code consommateur.

## Garanties conservées

Les trois refus d'authentification attendus sont exactement403 ; la tolérance401/403 de l'ancien pilote n'est pas reprise comme un assouplissement. Les preuves du certificat Envoy sont distinguées de celles du signataire distant. Un fichier de configuration ne peut pas s'auto-déclarer autorisé : seul le mécanisme existant effectivement identifié peut admettre la cible ou effectuer le stimulus et sa restauration. S'il manque, l'exécution reste refusée, sans création d'un nouveau backend.

Les scénarios conservent inscription achevée puis nouveau login, réassociation réelle, portée du Bearer au seul départ prévu et observations du même run. Aucun nouveau framework, serveur de preuve ou qualification Linux n'est ajouté.

## Preuves et limites

Le document récupéré est identique byte-à-byte à celui de l'auteur. `git diff --check` est sans erreur. Aucun test, appel applicatif, k6/Chrome ou runtime nouveau. La matrice consolidée est toujours attendue ; aucun lot de code consommateur n'est publié. Les observations précédentes de ports sans listener restent un prérequis manquant, pas une permission de réveil.

## Suite autorisée

Rapprocher la matrice avec le contrat, fixer les seules interfaces nécessaires, intégrer/ratifier puis confier les scénarios et l'adaptation indispensable du parent à des propriétaires distincts. Aucune nouvelle approbation de jalon n'est demandée ; une cible ou permission réellement manquante fera l'objet d'un arbitrage précis sans bloquer les travaux indépendants.

## Annexe technique

Contrat : `docs/contracts/k6-windows-auth-functional.md`,178lignes. Candidat auteur final `result_mvwjb3kq_0000_wlxmmg`, même session `ses_e6d836a47ffe5hX2epRUXT0AXF`. Les corrections finales correspondent au document déjà récupéré et vérifié ; pas de nouveau run pour remplir ce rapport. SHA256 `A5CB04791099551A2B850F3CF0731CE2EDA554717DB4EFD8E6CE0B34194AF682`.

L'intégration native n'avait aucun commit à fusionner ; le seul document non committé a été récupéré par patch et son identité vérifiée. L'arbre auteur reste préservé ; aucun retrait/cleanup tant que les livrables durables ne sont pas publiés et leur disposition vérifiée.
