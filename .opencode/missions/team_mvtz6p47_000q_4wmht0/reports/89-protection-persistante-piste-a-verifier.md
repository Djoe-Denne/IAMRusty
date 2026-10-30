# Protection persistante : une piste à vérifier, sans élargir les droits

Le but reste de tester les outils Windows dans leurs protections existantes.
L'architecte confirme que la fermeture du lanceur libère ses verrous temporaires,
mais que les permissions des fichiers peuvent continuer à bloquer leur modification
par des tiers non autorisés. Elles ne protègent pas contre un administrateur ou
un opérateur compromis ; cette limite était déjà explicite dans le modèle initial.

La piste retenue pour vérification est donc une protection persistante dans cette
frontière existante, pas une promesse de code universellement immuable. Si un
processus survit ou si sa fin manque, le bundle et les données restent réservés :
ni suppression, réextraction, remplacement ni réutilisation qui les modifierait
avant résolution vérifiée.

L'architecte formalise cette précision ; le reviewer vérifie séparément qu'elle
n'exclut aucun acteur précédemment couvert et précise la preuve ciblée nécessaire.
**Le gel des essais réels reste en place.** Aucun droit nouveau, arrêt forcé,
superviseur supplémentaire, test navigateur ou reprise WSL n'est admis par cette piste.

## État des preuves

- Avisarchitecte result_mvvemxzx_002l_awca0h, sourcebd7a2bf : possibilité conditionnelle,
  pas fermeture de la réserve ni qualification.
- Sourceinitiale68631c0 recoupée : guide65–68 et code115/127 déclarent déjà les rôles
  fiables et la limite contre opérateurcompromis. Ce constat n'élargit pas cette liste.
- RevueR-1initiale : acteur moinsprivilégié ayant un droitwrite surDLLpréexistante.
  La correspondance entre ce risque et l'assurance proposée reste à confirmer.
- Tests22 et préflight statique des vrais fichiers restent valides dans leur portée ;
  aucune version k6/Chrome Windows exécutée ni preuve d'isolation effective acquise.
