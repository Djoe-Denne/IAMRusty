# Refus privé corrigé ; contrôles de fermeture

## Correctif intégré

Le consommateur privé refuse désormais tout FATAL concurrent, même tronqué.
Le veto est volontairement conservateur : une mention supplémentaire du mot peut
empêcher un relevé, mais ne peut pas en autoriser un nouveau. Le classificateur
public et les formats positifs restent inchangés.

Les deux fichiers correspondent exactement au candidat et les **165 tests Windows
réussissent**, avec8skips POSIX. Le test de régression couvre même flux, flux
opposés et ordres inversés ; les oracles vérifient absence d'écriture et de
publication, pas seulement un résultat de helper.

## Suite sans collecte

Le reviewer reprend uniquement la fermeture du défaut. En parallèle, le lead
vérifie le code conservé sous Linux avec des fixtures connues. Les162tests Linux
précédents ne sont pas présentés comme une preuve de ce correctif.

L'architecte précise séparément l'opération du vrai parent ; ni cette proposition
ni le correctif ne créent un relevé. La lecture locale doit encore être organisée.
AIForAll reste arrêté jusqu'au besoin réel de tests et aucun lancement Chromium
n'est prévu pour ces vérifications.

## Preuves

- Source `cbab06eb68079f43ddb2246fc69c7dbdde68f071`, delta contre968592f.
- Lead Windows165/1.130s/exit0/skips8 ;20autres fichiers de base byte-identiques,
  fonctions du lanceur hors veto AST-identiques ; hook normal et diff-check0.
- Auteur : ROUGE3tests/32sous-cas en échec, GREEN3tests/38sous-cas, non rejoués
  par le lead. Aucun test ou site natif inconnu utilisé.
- Sauvegarde22fichiers et index vérifiée :
  `.tmp/k6-private-site-s1-source-cbab06e-6b4da7e6b5a24a509d8efc876fd782f7`.
- Brief de fermeture : `20261030-k6-private-site-s1-closure.md` ; pas de PASS de
  revue, preuve Linux ou qualification fonctionnelle encore acquis pour le correctif.
