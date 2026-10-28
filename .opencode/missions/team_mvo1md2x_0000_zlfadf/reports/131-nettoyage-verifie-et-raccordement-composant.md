# 131. Nettoyage vérifié ; raccorder uniquement la preuve après expiration

## Résultat du nettoyage

Le but reste la notification durable et sa reprise. L'audit demandé n'a trouvé
aucune modification de production allant contre la recommandation web : le
dispatcher SDK est inchangé depuis le pin, et les corrections HTTP sont utiles.
Il n'y a donc pas de correctif de production à retirer par présomption.

Les deux tests composant sont conservés : le premier vérifie explicitement
l'ancien propriétaire et son verrou pendant l'observation ; le second vérifie
la reprise après expiration. Supprimer le premier sans équivalence démontrée
réduirait la couverture. Aucun changement de leurs assertions n'a été fait.

La documentation de plateforme est réconciliée avec l'état actuel ; le contrat
sépare désormais photographie courante et chronologie historique non opératoire.
Les anciennes autorisations ne doivent plus être lues comme des consignes actives.
Les preuves et changements antérieurs sont préservés.

## Plan révisé et responsabilité

Le PM a été sollicité et repris dans sa même session, mais deux notifications
natives signalent l'absence de réponse finale récupérable. Aucun livrable PM
n'est accepté. Le lead a donc révisé directement le [plan courant](../implementation-plan.md).
Pas de changement de politique, de modèle ou de classification.

Ordre retenu : audit/nettoyage, vérification statique, une preuve composant après
expiration, puis bilan borné. Enquêtes répétées, drain facultatif et unlock sont
retirés du chemin actif ; les tâches et résultats historiques ne sont pas effacés.
La verticale finale, la publication SDK et le pin final restent séparés et bloqués.

## Vérifications réellement faites

- `git diff --check` : succès.
- Empreintes des deux tests et du dispatcher identiques aux sources précédemment
  compilées et revues : aucun nouveau Cargo de compilation nécessaire pour ce nettoyage.
- SDK local propre ; mêmes cinq chemins modifiés qu'avant le nettoyage.
- Lanceur corrigé identique à l'empreinte validée ; `node --check` : succès.
- Aucun processus Cargo observé ; configuration jobs24 et target E confirmée.
  Docker Desktop est déjà actif : aucun démarrage demandé ou effectué.

## Raccordement borné

La demande utilisateur de reconnecter l'exécution après nettoyage est appliquée
au seul composant après expiration, pas à une campagne générale ou à l'ancienne
question127. Avant départ : inventaire frais et bail des seules fixtures créées
par ce run. Une exécution ciblée du test revu ; échéance fixe <=55s depuis
reconstruction, cleanup6s/phase61s ; aucune modification produit ou reset de délai.
Le lanceur réutilise les fixtures, les images déjà présentes et le registre
append-only ; aucun build d'image, pull, Kind ou nouveau harness.

Les ressources préexistantes ne sont pas adoptées. Seules les créations attestées
du run peuvent être arrêtées ; retrait des seules fixtures possédées Exited(0)
selon la politique existante, jamais de prune ni suppression de volume. Un défaut
de préflight bloque le départ ; un échec ne déclenche pas de relance automatique.
La preuve après expiration reste absente à l'heure de ce rapport.

## Annexe technique

Source AI `1f2dd30`, SDK local `d60a88b`, pin committé129 inchangé.
SHA256 composant `445f729af1b350dc0e702cb3aa0fbec9715bbe1c2a8e7ae793d309a99fffd1ea` ;
vertical `6ac7be0e9e220a501241a0b48bf261c6fd7e14aa921ab098dbbfa29e7df071e1` ;
dispatcher `888769e031e367e6fa72a248230b071fa073a72c4413c194184eb7f995007124` ;
lanceur `a77dba1db77708532c4d381c5ab3ec2b8cca4bacceb788c0f541bdaed576554a`.
Les lectures de lanceur couvrent inventaires, création/ingestion du ledger,
cleanup par IDs, sélection exacte du test et bilan final. Les fixtures et leurs
effets déjà inspectés restent inchangés ; leur état réel sera observé à nouveau.
