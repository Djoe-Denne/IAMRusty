# Course corrigée : tests ciblés verts, relecture finale

Le correctif conserve désormais la version renvoyée par la modification réseau
elle-même. Une lecture ultérieure ne peut plus lui attribuer la règle d'un autre
écrivain. Une réponse perdue ou incohérente provoque un arrêt sans retrait forcé
ni nouvelle tentative incertaine.

Les **64 tests ciblés passent** : 52 pour le garde de commandes et12 pour
l'opération ingress. Le cas de concurrence précédemment rouge passe maintenant.
La nouvelle sortie JSON reste limitée à cette seule règle réseau ; les tentatives
sur des Secrets, d'autres objets ou des cibles détournées sont refusées avant
toute commande externe.

L'amendement de l'architecte est ratifié. La comparaison du code confirme que les
collecteurs réseau, les observations Admin, l'ancrage et le nettoyage n'ont pas
changé : seule la grammaire de commande est étendue. Les preuves d'origine restent
intactes. Leur correspondance exacte avec la nouvelle version de code n'est pas
encore activée : la relecture sécurité indépendante porte sur ce delta et sur
la paire proposée avant son approbation précise.

**Aucune règle réseau n'a encore été appliquée.** Après cette vérification,
je poursuivrai les contrôles réels de l'entrée, puis Envoy et navigateur, dans
le périmètre déjà autorisé.

Mise à jour : la revue indépendante a clos la course et le défaut Admin sur le
code livré. Le dernier complément de provenance a ensuite été aligné : format
immuable précis et refus d'une capture altérée sous une version historique.
Les treize tests de l'opération passent ; les52 tests inchangés du garde sont
réutilisés. La relecture porte maintenant uniquement sur ce petit complément,
toujours avant activation et sans résultat réseau revendiqué.

## Preuves techniques

- Livraison locale98bdd75, hook normal de formatage réussi ; aucun push application.
- Suites :52PASS/0skip15.643s ;12PASS/0skip4.572s ; exit0.
- Checkout de livraison réellement utilisé vérifié ; contrôle de source et cinq
  contre-exemples de périmètre réussis. Trois tests ciblés de grammaire/provenance
  y réussissent également, sans les compter comme de nouveaux tests distincts.
- Table de compatibilité encore vide ; proposition privée scellée, pas fichier
  runtime acceptant librement des empreintes. Aucun résultat réseau déduit des fakes.
