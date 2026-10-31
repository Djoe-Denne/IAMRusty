# La campagne d'intégration s'est arrêtée avant les tests

La campagne Manifesto n'a exécuté aucun test : l'éditeur de liens Windows a
signalé un manque de mémoire en construisant plusieurs binaires de test.
C'est un échec d'environnement, pas une preuve rouge fonctionnelle. Les tests
unitaires et la compilation des consommateurs déjà réussis restent valides.

L'inventaire est inchangé : 23 conteneurs avant et après, aucun conteneur de
fixture créé, aucune fuite. Le code et le lockfile sont restés identiques.

Je conserve les 24 jobs Cargo imposés et le modèle local résident. Je ne nettoie
pas les caches et n'arrête aucun workload pour forcer la compilation. La reprise
portera sur les deux binaires d'intégration prévus, un à la fois. Cela réduit le
nombre d'éditions de liens simultanées sans changer la configuration Cargo.

Pour conserver le témoin et le nettoyage éprouvés, le runner existant recevra un
sélecteur facultatif limité à ces deux binaires Manifesto. Son parcours par défaut,
ses fixtures et son contrôle de propriété restent inchangés. Ce n'est pas une
nouvelle infrastructure de tests ; le sélecteur sera contrôlé avant utilisation.
Une réussite de cette sélection ne sera pas annoncée comme une suite complète.

## Preuves et limites

- Échec initial : linker 64 bits, LNK1102, mémoire insuffisante ; zéro test lancé.
- Campagne it-0c33ce196e984a8f964ffacdd70b39fb, nettoyage sans problème.
- Après arrêt : 10,85 Gio disponibles sur 61,56 Gio ; engagement mémoire
  96,69 / 125,56 Gio. Ces mesures ne sont pas le pic de la compilation.
- Les erreurs ultérieures de chargement de crates ne suffisent pas à établir
  un défaut de code ; elles seront reconsidérées si elles persistent en sélection.
- Aucun réglage à moins de 24 jobs n'est appliqué ou implicitement autorisé.
