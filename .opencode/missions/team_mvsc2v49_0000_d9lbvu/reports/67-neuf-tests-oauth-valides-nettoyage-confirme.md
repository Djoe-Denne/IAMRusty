# Neuf tests OAuth validés, nettoyage confirmé

Les neuf tests d'intégration OAuth passent, dont la connexion après inscription
qui reproduisait le défaut. Aucun test ignoré ou filtré. Le test vérifie aussi
la signature, l'autorité configurée, la session persistée et le refus des rejeux.

La fixture PostgreSQL créée pour cette campagne est retirée. Il reste zéro
conteneur possédé actif ; les six conteneurs préexistants et leurs états sont
inchangés. La campagne et le registre de ressources concordent.

Je passe à la livraison Linux du seul service IAM corrigé et de sa migration,
en conservant les autres services et la configuration réseau du laboratoire.
La preuve finale dans Chrome reste à refaire après ce déploiement ; ces tests
d'intégration ne la remplacent pas.

## Preuves techniques

- Source7b8c1f95…, SDK02e5 ; lanceur existant, RUN_ENV=test, Windows24jobs/E:.
- Commande annoncée au rapport66 : exit0/33.670s ; neuf PASS, zéro failed/ignored/
  filtered ; durée tests28.43s.
- Campagne it-23a8bf3aee4a4aa5851b4b0c582602aa ; log
  `.tmp/oauth-authority-it-native-5e08087b59d54094bc6fad12967cb5b7.log`.
- Une création prouvée par ledger et observer, ID complet ecc22093… ; événement
  removed au registre `.tmp/docker-resources-20261029.log`, absence live confirmée.
- Inventaires6→6, nouveau non possédé0. Aucune suppression ou adoption étrangère.
