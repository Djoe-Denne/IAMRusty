# Code Windows intégré et tests ciblés réussis

Le but reste de qualifier k6 et le cas navigateur minimal sous Windows. Les sept
fichiers de l'auteur sont maintenant intégrés et leurs empreintes correspondent
exactement à sa livraison. Le lead a exécuté les tests sur ce contenu : **21 tests
réussis, aucun ignoré**. Les dix identités de processus de fixture sont observées
terminées et leur dossier temporaire a été retiré.

Ces tests vérifient le lanceur, pas k6 ou Chrome réels. Aucun navigateur, GET,
Docker, Kind ou WSL n'a été lancé. Le nettoyage incomplet du premier essai ancien
de l'auteur reste une limite distincte, sans réussite rétroactive.

## Version Chrome : solution compatible retenue

Les sources officielles confirment une API k6 qui donne la version du navigateur
contrôlé. L'architecte propose de la vérifier dans le même cas about:blank, sans
nouveau navigateur ni canal de débogage. Le lead a recoupé ces sources et ratifié
ce raccord minimal. L'auteur va ajouter le check et ses tests négatifs.

La commande Chrome `--version` reste **non exécutée et inconclusive** : les sources
ne garantissent pas un comportement Windows sans interface graphique. Elle ne
sera pas présentée comme réussie. La version réelle pourra être attestée par
l'API du seul navigateur possédé, si le futur cas et toutes ses gardes réussissent.
Les empreintes ou métadonnées seules ne remplacent toujours pas cette preuve.

En parallèle, une revue indépendante bornée portera sur les primitives Windows
stables de chemins, propriété des processus, isolation et nettoyage. Les vraies
campagnes attendent correction intégrée et fermeture des défauts bloquants utiles.
HTTP reste inconclusif sans cible existante admise ; Linux demeure suspendu.

## Preuves techniques

- Base principale7f2902ef + septsources intégrées noncommittées, hashes natifs
  vérifiés ; `git diff --check` global exit0.
- Commande réelle : `C:/Python313/python.exe -B -m unittest discover -s
  ops/tests/k6/tests -p test_*windows*.py` :21tests/30.695s/exit0/aucunskip.
- Journal privé courant :16lignes,10identitésPID/création distinctes,10finies,
  aucune observation nonfinie ; dossier de fixture absent, anciennes entrées conservées.
- Contratversionarchitecte reçu byte-identique SHA7af3137d…, puis ratifiéAccepted.
  Sourcesk6v2.3.0 mapping73–79/common787–807 et Chromium1561255–1267 recoupées HTTP200.
- PolitiqueQA révision2 non modifiée ; résultat Chrome--version NOT_EXECUTED, pas
  satisfaction inventée de toutes les commandes. Aucun push ou CI exécuté.
