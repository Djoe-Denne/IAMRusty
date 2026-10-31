# Oracles intégrés, vérifications finales avant les IT

Les derniers tests et la correction d'isolation des fixtures sont intégrés.
Les trois fichiers reçus correspondent exactement à la livraison auteur. Deux
helpers supplémentaires ont été réalignés sur la table réellement écrite par
l'outbox, sans changer leurs assertions : compter l'ancienne table vide aurait
donné une assurance trompeuse sur l'absence de mutation.

## Résultats déjà acquis

- Le test du délai cumulé du signataire passe, sans test ignoré ; il couvre les deux opérations avec un témoin positif sous budget.
- Le binaire des tests d'organisations compile après correction de la fixture ; aucun test PostgreSQL n'a encore été exécuté.
- Les sept oracles PostgreSQL sont maintenant présents : isolation/pagination, snapshot de suppression, deux tests du schéma initial, upgrade sans backfill, refus du downgrade et isolation de la fixture.
- Le formatage global passe lors du commit canonique. Les résultats antérieurs valides ne sont pas rejoués par principe.

## Phase actuelle

L'arbre est gelé. Un unique créneau Cargo vérifie les six composants, puis les
trois binaires Hive ciblés, un à la fois, et Clippy sur les packages convenus.
Cela évite le gros lot simultané ayant épuisé la mémoire. Les24 jobs restent
inchangés et aucun nettoyage de cache n'a été effectué.

La revue des oracles et la relecture infra du petit correctif Linux continuent
en lecture seule. Après ces prérequis, la campagne réelle utilisera les fixtures
existantes avec inventaire et nettoyage possédé. Aucun Docker, Kind ou navigateur
de mission n'est encore lancé ; aucun PASS fonctionnel global n'est annoncé.

## Annexe technique

- Canon :e03fddc5ece471803848721ff9e55aa1e23f9f7b, livraison testsbbeb7da parentff63cd4. SDK74e8 publié/pincé inchangé.
- Trois buffers reçus byte-exact ; deux corrections SQL root dans organization_signer_permissions.rs et member_update_api_tests.rs ciblent rustycog_outbox_events du vrai OutboxRecorder.
- Sept logs/métadonnées archivés avec comparaisonSHA256 sous `.tmp/auth-oracles-pool18-proof-bbeb7da`. LogsCargo UTF16LE décodés strictement.
- Auteur : no-runfinal17.62s, zéro test ; T4remote1PASS/0ignored/19filtrés,13.59s test. Ne pas additionner les sélections superposées.
- Merge natif réussi, branches conservées/cleanup pending. Tâche des oracles reste ouverte jusqu'aux preuves PG, pas acceptée sur une compilation seule.
- Gate séquentiel actuel : shell sh_184d22ef0001a15gJrUbcFa428. Attendre sa notification, pas de polling ou de deuxième Cargo.

### Résultats reçus

Les cinq contrôles séquentiels réussissent : checkdes six composants, trois
binairesHive compilés et Clippy. Aucun épuisement mémoire ni avertissement des
deux lints ciblés ; les autres avertissements restent visibles.

La revue indépendante ferme les quatre lacunes EN SOURCE, sans nouveau finding.
La relecture infra valide aussi le correctifLinux statique, sans prétendre avoir
construit les images. Un ancien test d'absence d'authentification, déjà non
enregistré avant cette mission, est réactivé sans modifier ses deux assertions401.
La liste réelle confirme23 tests enregistrés, mais toujours zéroPG exécuté.

Canon79428d1. La campagne réelle peut maintenant être préparée selon les règles
d'inventaire et de propriété ; aucune qualification finale n'est anticipée.
