# Lecteur de dépendances corrigé, 27 tests validés

Le défaut du lecteur ponctuel est corrigé : une ligne de chargeur ne couvre son
nom de dépendance que si son chemin a déjà été reconnu et autorisé. Les doublons,
chemins inconnus et dépendances non couvertes restent refusés.

Le lead a reproduit le défaut avant correction : deux tests, quatre sous-cas en
échec, sortie1. Après application du delta exact, les27tests passent sans skip,
sortie0. Les empreintes du script et des fixtures sont identiques à la livraison.

**Cela ne modifie aucun résultat passé.** Le vrai run ldd conserve son compte
inconnu, puisque ses flux privés ont été effacés. Les copies de référence de ce
run sont préservées ; le correctif est dans une nouvelle copie temporaire. Aucun
nouveau `ldd`, `unshare`, navigateur ou changement système n'a été lancé.

Le contrat actuel borne encore chaque commande à une exécution par voie. Une
éventuelle mesure ldd supplémentaire fait donc l'objet d'un arbitrage ciblé avant
implémentation de son consommateur ou runtime. Elle ne sera pas autorisée par le
seul succès des tests ni cachée sous un changement de nom de voie.

Dernier état runtime vérifié : AIForAll arrêtée, baux libérés, copies retirées ;
ARTIST arrêtée et docker-desktop actif inchangés. Chromium et HTTP non qualifiés,
cause Chrome inconnue, aucun push/dispatch implicite.

## Preuves

- Candidat corrigé `result_mvv8na90_000z_ahzrw8`.
- Lead ROUGE :2tests/0.001s/exit1/quatre sous-cas ; GREEN complet :27tests/0.254s/
  exit0/sansskip,27comprend21tests auteur et six fixtures indépendantes du lead.
- SHA256 script : `048d9c9608409610b2ff989f6052c7e272e09cdcf75c0ae047516ee658bc3083`.
- SHA256 fixtures : `0d0f230aa7ed23a8d394072255b700b9dcdd4c292b1c3ed0cacb71fb3d7afb20`.
- Copie corrigée : `.tmp/k6-native-probes-candidate-048d9c96` ; référence7f17/09da
  intacte dans la copie précédente, validateur du résultat historique inchangé.
