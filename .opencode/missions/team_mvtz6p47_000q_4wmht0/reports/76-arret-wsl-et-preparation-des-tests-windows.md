# Arrêt de WSL et préparation des tests Windows

L'utilisateur demande de suspendre cette approche, d'archiver les outils créés
pour elle et de tester maintenant sous Windows. La qualification Linux sur une
machine GitHub Actions fera l'objet d'une autre mission.

## Décisions appliquées

- La seconde mesure ldd est abandonnée avant exécution. L'auteur a reçu l'ordre
  de geler son raccord temporaire à une frontière sûre, sans nouveau test ou run.
- WSL est **INCONCLUSIVE** pour la qualification navigateur ; les tests unitaires
  et la réussite userns conservent leur portée propre, pas un PASS Chrome.
- L'instance AIForAll, son VHDX surI:, ses données et ses outils installés seront
  conservés. Aucun retrait forcé, purge, changement d'ACL ou arrêt étranger.
- Les sources publiques utiles seront archivées avec leur provenance, sans les
  journaux privés, profils, entrées de campagne, baux, caches ou exécutables.
  Premièrearchive créée :19sources en copies textuelles inactives byte-identiques,
  manifesteSHA256, hors découverte tests/CI ; originaux récupérables conservés.
- Aucun lancement GitHub Actions, nouvelle VM, Kind/Docker ou missionLinux n'est
  déclenché implicitement. Les scénarios restent mission1, pas OAuth ou charge.

## Suite et frontière de validation

Le lead prépare les archives et inventorie les outils Windows sans exécuter de
test. L'architecte prépare dans sa session existante le raccord Windows minimal,
en réutilisant les scénariosJavaScript et sans réécrire les protectionsPOSIX pour
les faire paraître portables. Un résultatWindows restera distinct d'un résultatLinux.

La QA approuvée de la mission contient des commandesLinux et l'obligation d'une
preuveLinux. Elle doit être révisée pour les essaisWindows : niveau proportionate
et catalogueOpenAI inchangés. Le brouillon exact sera présenté parconfiget
approuvé parstartavant l'admission de nouveaucodeou testsWindows. Ce n'est pas
un nouveauGOà chaquejalon ; c'est le changement réel de cible et de critères.
Les opérations d'arrêt, conservation et préparation documentaire restent permises.

## Preuves et limites actuelles

- Base principale915042f, sourceOPSf279 inchangée ; contratlddsupplémentaire
  ratifié mais non consommé, quota absent au dernier contrôle.
- MesuresréellesWSL :unshare1/exit0 ;ldd1/exit0/countnull ; aucunChrome depuisreprise.
  DernierChrome réel avantcas/FATAL, aucunecause démontrée.
- Dernierinventaire nonréveillant :AIForAllStopped/ARTISTStopped/dockerRunning ;
  ressourcesétrangères inchangées. Étatà recontrôler avant toute opération.
- Septarbres et sixrefspréservéesmission encore présents, zérocommitunique ;
  sauvegardes/dispositionshistoriques, pas de retrait ouarchivageéquipe revendiqué.
- InventaireWindows nonexécutant :k6.exe présent dansProgramFiles, version de
  fichier non renseignée ; Chromeinstallé149.0.7827.53, pas le pin156actuel.
  CeChrome n'est pas adopté automatiquement ; aucunbinairek6/Chrome exécuté.
- [Archive inactive et manifeste](../../../../docs/archives/k6-wsl-20261030/README.md).
- Rapport75 devient historique : sa poursuite envisagée est remplacée par la
  présente instruction utilisateur, sans effacer ses décisions ou preuves.
