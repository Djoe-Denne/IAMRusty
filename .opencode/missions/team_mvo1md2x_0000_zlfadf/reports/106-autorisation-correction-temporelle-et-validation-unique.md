# Corriger la vérification au bon moment

Après le diagnostic105, l'utilisateur choisit **« Corriger le contrôle temporel »**. Il autorise une correction uniquement du moment de vérification dans le test vertical, un examen indépendant de la préservation des garanties, puis une seule exécution complète du scénario panne/reprise.

## Contrat de correction

- AI1f2/SDKd60, production/configuration/délais/backoff/sampler/visibilité inchangés. Aucun SDK edit, pin ou publication.
- Test-engineer pool14, même session et arbre isolé, reprend sa source instrumentée archivée. Le diagnostic105 reste immuable. Seul fichier vertical modifiable ; retirer ou limiter l'instrumentation temporaire si nécessaire, sans nouvelle infrastructure ou dépendance.
- Observer l'échec dans la future SQL déjà possédée, sous la même limite absolue de 25 secondes et en parallèle du sampler inchangé. Vérifier immédiatement que la prochaine tentative est future à cet instant, pas après la barrière d'observation de 12 secondes. Pas substitution par une ancienne heure artificielle ni simple next_at > updated_at.
- Conserver les contrôles d'identité, persistance, présence d'erreur, tentatives, reconstruction, replay, unicité des droits et HTTP403/200. Une ligne absente ou une erreur d'observation ne peut pas devenir un succès silencieux. Ne pas imposer que tout l'état reste figé si une nouvelle tentative légitime se produit.
- Formatage/compilation sans exécution autorisés dans le slot Cargo Windows unique, 24 jobs configurés. Retour diff exact/hash et raisonnement observable de préservation du contrôle ; pas de test exécuté à l'étape auteur.
- Lead archive et intègre le résultat avant la revue indépendante fresh bornée au test et à ses risques. Briefing persisté avant corrections. La revue ne redéfinit pas l'architecture ni la portée.

## Exécution séparée après examen

Une seule validation verticale autorisée après intégration et revue, préflight/bail runtime et sourcehash exact. Même lanceur et fixtures, aucune répétition jusqu'au vert ou suite NoOp redondante. Diagnostic précédent consommé ; cette nouvelle exécution est une permission distincte. Si nouvel échec, conserver le rouge et rendre les faits, sans correctif de production automatique. Publication toujours soumise aux gates existantes ; ce choix n'autorise pas à publier.

Ressources précédentes préservées : dix conteneurs préexistants à la campagne105, SMTP nouveau arrêté ASK, deux volumes opaques ASK ; aucune adoption par nom ni arrêt global Docker/WSL. Inventaire frais requis avant nouvelle campagne, nouvelles identités tracées et zéro possédé actif à la fin.
