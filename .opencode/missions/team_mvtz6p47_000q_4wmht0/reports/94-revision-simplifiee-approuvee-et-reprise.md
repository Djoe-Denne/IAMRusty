# Révision simplifiée approuvée : reprise vers le premier cas réel

L'objectif immédiat est d'exécuter k6 avec le Chrome épinglé sur `about:blank`,
vérifier `6 * 7 === 42`, puis fermer les processus du test. L'utilisateur a approuvé
la révision qui retire les verrouillages personnalisés, les contrôles d'ACL et
l'attestation du sandbox. Ces mécanismes ne seront ni complétés ni remplacés.

La reprise suit trois étapes utiles : consigner le remplacement des anciennes
clauses dans le contrat, simplifier le code et ses tests couplés avec un seul
auteur, puis effectuer le premier essai réel après contrôle court du delta.
Aucune nouvelle revue de risques hypothétiques ne précède cet essai.

Les versions épinglées, le sandbox natif de Chrome, les profils temporaires séparés,
le timeout et le nettoyage raisonnable restent conservés. Un échec réel sera
diagnostiqué à partir des logs privés bornés, du code retour et des processus
observés. WSL/Linux restent non qualifiés ; aucune infrastructure n'est démarrée.

## Disposition des travaux précédents

Le premier livrable Windows est intégré et ses preuves ciblées sont archivées.
Son ancienne tâche est disposée comme source livrée puis remplacée par la décision
utilisateur, pas comme campagne Windows réussie. Une nouvelle admission portera
la simplification sous la révision3. Les auteurs existants sont réutilisés lorsque
le routage natif le permet ; aucun modèle n'est substitué manuellement.

Le lot historique de qualification Linux reste différé : il n'autorise ni reprise
WSL ni exécution GitHub dans cette révision. Les sources historiques restent conservées.

## Références et limites

- Révision3 approuvée, hash `82a5aefa6a4dcd7deb05121d85b35da21b567585b615661f568ff45d27a22bb6`.
- Brief complet : `.cursor/review-briefings/20261031-k6-windows-simplification.md`.
- Base vérifiée précédente : `96142df` ; sources Windows intégrées `bd7a2bf`.
- Aucun k6/Chrome Windows réel encore exécuté. Les22tests antérieurs ne constituent
  pas une preuve du code simplifié, du sandbox ou de cette première campagne réelle.
- Aucun nouveau GO intermédiaire demandé ; intégration des entrées avant consommateurs.

## Préparation effectivement réalisée

L'ancienne tâche de livraison source Windows est disposée sur le tableau ; la
campagne réelle n'est pas déclarée réussie. Le routage natif a affecté la nouvelle
tâche documentaire à un nouvel architecte plutôt qu'à l'ancien membre disponible ;
aucun modèle ni fournisseur n'a été forcé. Le consommateur code attend son entrée
documentaire intégrée, pas une autre décision utilisateur.

Les documents de recentrage et la politique approuvée sont intégrés dans `9eb3658`.
Le hook normal de formatage et `git diff --cached --check` ont réussi, sans désactivation.
Les exécutables et archives déjà préparés sont toujours présents, ainsi que
PowerShell Windows et Python ; aucune installation ou exécution k6/Chrome n'a été
faite pour ce relevé. Cela ne remplace pas le contrôle des empreintes avant usage.

La proposition de conservation abandonnée est recoupée avec le texte reçu :
SHA256 `6652566a6f76ab8f9589f3055364805f557e0297ac07ff1e25ccbba3b498cab9`.
Cette vérification documente sa disposition, pas une nouvelle exigence de protection.

## Contrat simplifié intégré et ratifié

L'amendement de l'architecte a été lu et intégré. Les deux documents sont comparés
byte-identiques au candidat avant ratification : ils remplacent explicitement les
exigences rejetées, sans changer les versions, les assertions ou les limites de mission.
Le lead a ensuite ratifié l'amendement ; la proposition de conservation persistante
est abandonnée et ne gèle plus les essais. Le consommateur code peut maintenant partir.

Le mécanisme natif de fusion n'a trouvé aucun commit dans la branche documentaire,
dont les changements sont non committés. Les deux fichiers ont donc été récupérés
par patch ciblé puis recoupés par leurs SHA256. L'arbre source reste préservé ;
aucune suppression ni perte de ses changements n'est autorisée par cette intégration.

Preuve : candidat `result_mvw5ppw0_0001_m1d2jm`, contrat SHA256
`c599205c80526164cdf0f2241692caaf226e6e885fcc91769d044d16e3c9b9ba` et notice
`4398bc6986618765d7e9d16cfd8372abfb3b35743d267acffd38f4a8bf9f6cd6`, avant ratification.
Aucun nouveau test ou vrai outil exécuté pour ce jalon documentaire.
