# Correction ciblée : sortie réelle k6.exe sous Windows

## Défaut observé, pas hypothétique

Premier mode navigateur réel exécuté par le lead après intégration byte-identique
des9fichiers du candidat `result_mvw6lbki_0000_4m8tgj`. Résultat parent exit2,
`VERSION_INVALID`, avant les inspections et avant tout Chrome.

Le binaire officiel épinglé et rehashé a réellement répondu :

`k6.exe v2.3.0 (commit/e088784614, go1.26.8, windows/amd64)`

Son exit natif est0, stderr vide, capture complète ; job total1/active0, unique
identité créée observée terminée et image égale au k6 attendu. Inventaires avant/
après :0/0 k6 ou Chrome. Le parent indique cleanupPASS/confidentialityPASS.
Le parser `Assert-K6Version` exigeait le préfixe `k6 ` : c'est la cause démontrée.

## Travail autorisé, même auteur et même tâche

Ne modifier que `lib/windows.ps1::Assert-K6Version` et le test Windows ciblé de ce
contrôle. Reprendre le même membre23/session/tâche/modèle, fichiers courants conservés.
Pas de nouveau contrat, revue, garde, framework, sonde, installation ou enquête.

1. Ajouter un test utilisant exactement la sortie observée et établir son échec
   contre le parser courant, sans exécuter le vrai k6.
2. Accepter seulement `k6` ou `k6.exe` comme nom initial, en conservant tous les
   contrôles existants de version exacte2.3.0, commit hex, versionGo et windows/amd64.
   Le point de `.exe` doit rester littéral, pas un joker. Conserver les négatifs utiles.
3. Exécuter les tests ciblés du parser, puis livrer immédiatement les deux fichiers
   ou le petit delta exact, SHA256 et résultats réellement exécutés. Pas de suite
   redondante ou de nouvelle commande réelle par l'auteur.

Le lead reprend ensuite le mode browser, mêmes versions et scénario about:blank/42,
nouveau répertoire de run, sans autre GO utilisateur. Les logs privés du premier
échec sont conservés hors Git ; ne pas les recopier dans le livrable.

Références : rapport96 de mission, contrat Windows amendementrévision3Accepted,
ledger canonique `C:/Users/djden/source/repos/AIForAll/.cursor/handoffs/20261029-k6-01-socle.md`.
La politique catalogue approuvée reste inchangée. Aucun Chrome ou GET n'a encore été
exécuté ; les32tests d'auteur précédents ne couvraient pas ce préfixe réel.
