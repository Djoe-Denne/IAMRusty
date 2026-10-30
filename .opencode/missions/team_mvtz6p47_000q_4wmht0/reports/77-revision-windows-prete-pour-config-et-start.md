# Révision Windows prête : approuver les contrôles de la nouvelle cible

L'objectif demandé est maintenant de tester k6 et Chrome sousWindows, après
archivage de l'approcheWSL inconclusive. Les19sourcesWSL sont archivées et leurs
empreintes vérifiées dans le commit local. L'instanceAIForAll et ses données surI:
restent conservées et arrêtées au dernier inventaire. Aucun nouvelessaiWSL,
Chrome, k6 ou GitHubActions n'a été lancé.

## Ce qui est prêt et ce qui reste à faire

Le contratWindows de l'architecte est intégré comme Proposed, sans code exécuté.
Il prévoit un petit lanceurWindows réutilisant les deux scénariosJavaScript,
les mêmes vérifications de résumé, des sorties privéesbornées, l'isolation du
navigateur et le retrait des seuls processus possédés. Un simple démarrage ou
un testJavaScript vert ne remplacera pas les preuves manquantes.

Le brouillon de mission propose11commandes et18critères de vérificationWindows.
Les modèlesOpenAI, l'autonomie longrun et la QAproportionate sont inchangés.
La seule révision de fond est la cibleWindows et ses contrôles, avecLinux/GitHub
remis à une mission future. Le contrat approuvé actuellement resteLinux : il faut
donc présenter et approuver cette révision native avant l'admission du nouveau
code et des testsWindows, sans demander ensuite un nouveauGO à chaque étape.

## Portée recommandée

1. Vérifier les outilsWindows épinglés, puis leurs versions et les deux scénarios.
   k6.exe est présent mais sa version n'est pas encore prouvée. Le Chrome installé
   est149, pas le pin156 : il n'est pas substitué silencieusement. Acquisition
   portable officielle et bornée seulement si les octets conformes manquent.
2. Exécuter le casChromium minimal surabout:blank, avec contexte neuf, isolation
   active, contrôles réels et nettoyage. VerdictWindows distinct deLinux.
3. TesterHTTP et les négatifs réels uniquement si une cible existanteautorisée
   est disponible ; sinon HTTPINCONCLUSIVE, sans nouvelle stack, tunnel ou serveur
   factice. AucunOAuth, charge, modification des services ou repriseWSL.
4. Avantpublication, suspendre les deux jobs du workflowk6Linux existant pour
   éviter une relanceUbuntu au pushWindows. La CI sera SUSPENDED/NOT_EXECUTED,
   pas qualifiée, même si GitHub afficheSuccess pour un job ignoré.

Cette option poursuit la demande sans ajouter de nouvelle infrastructure. Les
primitivesWindows d'identité, d'isolation et de nettoyage restent à implémenter
et à vérifier ; aucun délai fiable n'est annoncé avant ce travail. Refuser ou
différer la révision conserve les archives et la suspensionWSL, sans lancer les
essaisWindows ni faire passer un résultat manquant pour une réussite.

## Décision demandée

Recommandation : approuver cette révisionWindows bornée. Exécuter
`/ensemble-mission config` pour afficher le brouillon exact, puis
`/ensemble-mission start` si l'affichage convient. Le lead ne lance passtartà
la place de l'utilisateur. Un refus ou un report laisse la préparation en attente,
sans déclenchement de tests ou d'actionsGitHub.

## Annexe technique et preuves

- Archive locale `eb41a57` :19sources/182087octets, SHA256 des19blobsGit vérifiés,
  vrai hookcargoFMT et cached-diffcheck0. Aucunpush ou archivage natif final acquis.
- Contrat `docs/contracts/k6-windows-functional.md`,194lignes, byte-identique au
  candidat `result_mvva0f4z_0008_j1cmsh`, SHA256
  `32bb3c02006988db9d4c35a76750041027dc64716964b035d5d30a65dbd5732a`.
- Entrée prévue `ops/tests/k6/run-windows.ps1` ; les commandes du brouillon ont
  été alignées sur cette proposition, pas sur le lanceurLinux ou un scriptPython
  provisoirement envisagé. Aucun fichier de productionWindows n'est ajoutéici.
- JSON de politique valide, version2/catalogue4paires/threshold50inchangés,
  OpenAIseulement, proportionate etlongrun conservés ; activationnonacquise.
- SourcesJS actuelles relues :about:blank/évaluation42, compteurs stricts et
  origineHTTP littérale127.0.0.1:18080 ; aucune modificationJS.
- AuteurWSL13 : arrêtgracieux demandé, fichiersconservés, livraison de son état
  au gel encore attendue. Elle ne doit ni être consommée ni lancer de secondldd.
- Worktrees/branches temporaires encore conservés et WIP étrangerspréservés ;
  mission nonclôturée, aucun retrait forcé, purge ou donnéeprivée publiée.
