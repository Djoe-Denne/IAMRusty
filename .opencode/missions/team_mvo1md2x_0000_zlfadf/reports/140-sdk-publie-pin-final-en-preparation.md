# 140. SDK exact publié ; pin de livraison préparé

Après la validation verticale139, le SDK testé et revu a été publié avec succès
sur la branche de mission existante, sans force et sans modifier sa branche
principale. `ls-remote` confirme le SHA exact distant.

Cette étape est couverte par l'autorisation initiale53, qui incluait le correctif
HTTP, ses tests et sa publication/pin. Aucun nouveau mécanisme produit ou
extension de QA n'a été ajouté.

## Faits vérifiés

- Dépôt distant confirmé comme `Djoe-Denne/rustycog` avant push.
- Branche `mission-telegraph-ac7-zlfadf` : ancien129dcc1, nouveau
  `d60a88b2f2f499768a807941a1b4c80c9a366b3f`, publication fast-forward exit0.
- Vérification distante exacte après publication : mêmeSHA.
- SDK local propre ; aucun checkout ou fichier du dépôt frère utilisateur changé.
- Index application initialement vide, HEAD1f2dd30 inchangé ; seuls les cinq
  chemins de mission ont été préparés. Gitlink mode160000 désormaisd60 dans
  l'index, pas une copie de sources SDK.
- Diff préparé : deux documents, deux cibles de tests, gitlink ; contrôle
  whitespace0. Sources tests identiques à celles compilées/revues/exécutées.

Le commit application n'est pas encore affirmé dans ce rapport. Il utilisera le
hook normal `.githooks/pre-commit` (`cargo fmt --all -- --check`), avec slotCargo
unique. En cas d'échec, préserver l'index et les preuves, aucun --no-verify,
reset/stash ou formatage global automatique pour contourner le problème.

## Limites conservées

Vert fonctionnel ciblé au nominal et après panne, composant post-expiration et
revues ciblées ne signifient pas qualité globale sans dette. Clippy strict
historique reste non vert, Sonar/CI distante non exécutés pour cette tranche.
Pas de changement de statut ADRProposed ni promesse exactly-once/fencing.
Ressources possédées toutes arrêtées/retirées, SMTP nonzero/volumes ASK conservés,
workload étranger préservé. La publication application distante n'est pas couverte
ou affirmée par la publication du SDK.

Annexe commande exécutée :
`git -C rustycog push origin d60a88b2f2f499768a807941a1b4c80c9a366b3f:refs/heads/mission-telegraph-ac7-zlfadf`.
La sortie Git standard arrive sur stderr et PowerShell la présente comme
NativeCommandError ; le code commande0 et ls-remote exact confirment le succès,
ce n'est pas un défaut de publication. Log privé sdk-publication139.log sous
TI/.tmp/vertical-final134-inputs. Localisation : publication SDK et pin160000.
Impact : SHA rendu disponible pour les consommateurs ; commitAI encore à vérifier.
