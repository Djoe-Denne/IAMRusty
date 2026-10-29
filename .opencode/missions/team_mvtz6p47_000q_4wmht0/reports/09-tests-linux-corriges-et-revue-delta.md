# Les 40 tests du dispositif passent sous Linux

## Objectif et résultat

Le défaut de validation des répertoires découvert au premier run Linux est corrigé et intégré. Le lead a rejoué les tests sous Ubuntu24.04/WSL2 avec le compte non-root existant :40tests exécutés, exit0, aucun échec, erreur ou skip. Les six tests POSIX/processus sont maintenant réellement exercés.

## Ce qui a changé

Le répertoire de sortie valide son propre propriétaire et ses droits privés ; un fichier exige toujours un parent immédiat privé. La sortie doit toujours être neuve, canonique, non liée et hors Git. Le grand-parent public usuel n'est plus un motif de rejet accidentel. Les contrôles de sentinelle et d'overflow attendent leur diagnostic exact pour ne pas passer sur une autre erreur.

Ce delta de trois fichiers a été vérifié identique au candidat auteur avant capture. Le commit réel et son hook de formatage ont réussi sans désactivation de contrôle. Les garde-fous de cible, l'authentification et la CI sont inchangés.

## Suite et portée

Une relecture indépendante limitée à cette garde sensible est demandée. En parallèle, le lead peut préparer les outils épinglés sous son bail Linux, sans requête applicative et sans assouplissement système. Aucun nouveau pipeline QA ou GO de jalon.

Ce résultat ne qualifie pas encore les binaires k6/Chrome exécutés, la sandbox Chromium ni HTTP. La preuve Linux native/CI reste distincte de l'essai WSL. L'exposition publique et les négatifs HTTP restent non exécutables dans la V1 actuelle.

## Références techniques

- Commit db352be1ebe01eb275fd74d8cdee6f7298b99bf1 ; delta depuis db16aa7 : trois fichiers,123insertions/6suppressions. Pas de push.
- Commande /usr/bin/python3 -B -m unittest discover -s ops/tests/k6/tests -p test_*.py sous WSL ARTIST-Ubuntu/artist UID1000, cwd canonique monté ; exit0/Ran40/OK,0skip.
- Log contemporain .tmp/k6-linux-unittest-1793309664423.log. Les logs privés ne seront pas committés.
- Briefing indépendant .cursor/review-briefings/20261029-k6-private-directory-delta-db352be.md.
- Bail parent .tmp/k6-linux-qualification-mvtz6p47.json ; aucun container Docker créé/redémarré, aucune cible adoptée, aucun k6/Chrome exécuté à ce jalon.

## Préparation des outils et source Linux propre

Les archives épinglées ont ensuite été acquises et vérifiées par le provisionneur explicite sous supervision ; l'enfant a terminé avec cleanup vérifié. Aucun binaire k6/Chrome n'est encore exécuté.

Le préflight Git a refusé la provenance depuis le checkout Windows monté, vu comme appartenant à root. Aucune exception safe.directory ou modification de propriétaire globale. Une nouvelle copie propre du commit exact, sans les WIP, a été transférée dans un répertoire Linux privé appartenant à UID1000. HEAD, propreté Git et identité des16fichiers pertinents sont vérifiés. Cette adaptation de checkout ne modifie ni le produit, ni les gardes, ni le workflow.
