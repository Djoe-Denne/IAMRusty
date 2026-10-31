# Derniers oracles : protéger aussi l'isolation des tests

Les deux défauts de sécurité relevés dans le code sont désormais clos par la
revue indépendante. Le nouveau test du délai cumulé du signataire réussit ; le
binaire des tests d'organisations compile sans démarrer de base de données.
Les preuves PostgreSQL restent encore à exécuter.

Le contrôle des effets du harness a révélé un problème de tests à corriger avant
ce lancement : la migration de provenance interdit volontairement le retour en
arrière, mais l'ancienne remise à zéro des fixtures essayait précisément ce
retour. La bibliothèque ignore ensuite ce refus et réutilise la même base ; les
tests pourraient donc conserver les données du précédent cas et devenir trompeurs.
Ce constat est statique, pas un échec fonctionnel déjà exécuté.

## Choix borné

Conserver intégralement le refus de retour arrière en production. Adapter
uniquement le descripteur privé des tests Hive pour recréer le schéma de leur
base jetable, selon l'API existante de SeaORM. Cette opération ne sera exécutée
que sur les PostgreSQL créés et tracés par les fixtures de la campagne finale ;
aucune base vivante, ressource inconnue ou donnée utilisateur n'est concernée.

Un oracle vérifiera la remise à zéro effective et la présence du nouveau schéma.
Les tests de conservation des données lors de l'upgrade et de refus du downgrade
restent séparés. Pas de nouveau harness, dépendance ou changement de migration
de production ; l'auteur des tests conserve ce complément dans la même session.

## État et suite

Le lot se termine avec cette correction locale, puis sera compilé et intégré.
La revue des oracles relira le diff final avant les IT. Kind, Envoy réel et le
navigateur restent après les IT finales. Aucun nouveau GO global n'est demandé.

## Annexe technique

- Contrôle transitive : Hive tests/common.rs72–78 ; SDK rustycog-testing/common/database.rs93–106/125–131. Migration owner_issuer down refusé inchangé.
- SeaORMmigration1.1.20 exact résolu ; migrator.rs193–215/275–357 lu : fresh recrée sans down, refresh utilise down et ne convient pas.
- Ownership additionnel : seul run_migrations_down de HiveTestDescriptor, son commentaire, et oracle fixture_schema_reset_clears_rows_and_reapplies_owner_provenance dans organization_api_tests.rs. Aucune opération SQL exécutée par cette préparation.
- Résultats provisoires auteur : compilationHive10.68s/exit0 ; remoteT4 1PASS/0ignored/19filtrés. Logs à archiver à la restitution finale, pas d'exécution des sept tests PostgreSQL déduite de ces résultats.

### Préparation de la livraison Linux, sans démarrage Docker

Un autre écart de configuration a été localisé : le build Linux copiait le
réglage du répertoire de compilation Windows, mais cherchait les binaires dans
son cache Linux. Le Dockerfile fixe maintenant explicitement son répertoire
Linux et verrouille le Cargo.lock, sans modifier les24 jobs ni la configuration
Windows. Le contrôle source échoue avant ce patch puis passe ; la suite statique
concernée compte23 succès. Cela ne prouve pas encore un build Linux.

La livraison utilisera un worktree propre des seuls commits vérifiés, pas le
répertoire canonique contenant les fichiers privés temporaires. Relecture infra
ciblée demandée ; aucun build, image ou runtime créé par cette préparation.
