# Scellement sans privilège d'audit supplémentaire

Le deuxième bootstrap s'arrête lui aussi avant Kind, cette fois sur une condition
que le premier test ne reproduisait pas : le répertoire est créé avec une liste
de droits protégée. Aucun nœud ni volume créé ; les 20 conteneurs préexistants
restent inchangés.

Le diagnostic natif montre que `Set-Acl` réclame un privilège d'audit absent,
alors que le contrat ne modifie que le propriétaire et les droits d'accès.
Les deux outils d'exécution sont non élevés et ne possèdent pas ce privilège.
Je ne change ni compte, ni privilège, ni outil pour faire passer l'opération.

Le test reproduit maintenant exactement la création protégée du bootstrap et
échoue avec l'ancien appel. Le correctif utilise l'API .NET existante pour
appliquer les seules sections modifiées, avec les mêmes droits finaux. Il ne
demande pas d'accès à la politique d'audit. La suite passe ensuite ses 24 tests,
dont la vérification native des droits exacts du répertoire.

Ce complément est soumis à la même revue sécurité avant intégration. Aucun
troisième bootstrap n'est lancé à ce stade ; les deux essais et leurs ancrages
vides sont conservés. Images, IT et entrées OAuth privées restent disponibles.

## Preuves et limites

- Régression : le même oracle natif avec `mkdir(mode=0o700)`, rouge avant correction.
- Validation finale : 24 PASS, 2.243 s, pas de skip ; `.tmp/auth-windows-protected-dacl-exact-suite.log`.
- Droits observés : propriétaire courant, héritage désactivé, deux entrées Allow FullControl pour opérateur/SYSTEM, aucune autre entrée sur le répertoire propre.
- La reprise de bootstrap, Envoy et le navigateur ne sont pas encore qualifiés.

## Revue reçue et reprise explicite

La revue indépendante confirme la persistance Owner/Access seule et l'absence
de changement d'audit ou de privilège. Le delta exact est intégré, avec les
tests verts. Un troisième bail neuf est préparé pour la reprise explicite du
bootstrap existant ; aucune reprise des deux ancrages échoués.
