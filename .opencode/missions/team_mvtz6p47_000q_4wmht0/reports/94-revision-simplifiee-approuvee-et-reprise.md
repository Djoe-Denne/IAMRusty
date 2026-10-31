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
