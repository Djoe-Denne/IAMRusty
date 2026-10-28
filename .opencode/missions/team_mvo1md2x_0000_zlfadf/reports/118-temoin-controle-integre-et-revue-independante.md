# Témoin contrôlé intégré, revue de sa validité avant exécution

## Étape atteinte

Le même auteur a préparé le témoin117 en raisonnement natif high. Deux compilations sans exécution ont réussi, la seconde après ajout de compteurs pour ne pas confondre une opération annulée avec un résultat connu. Aucun test ni runtime exécuté.

Le lead a inspecté les328lignes du patch puis intégré exactement le fichier compilé. Il ajoute316lignes de test ; les anciens tests, le publisher suspendu existant et leurs assertions restent inchangés. Empreinte intégrée identique, diffcheck réussi, douze artefacts archivés avec copies vérifiées. Ni source de production, ni SDK, ni scénario vertical modifié par ce témoin.

Le protocole suspend une vraie publication après claimSQL terminé, annule et joint son propriétaire, puis pilote de vraies sélections du dispatcher reconstruit. Il mesure une fenêtre de25s incluse dans le verrou30s constaté. Les compteurs de cycles et claims, les états SQL et une observation SQS permettent de rejeter un faux succès par simple inactivité. L'annulation est une frontière de composant explicite, pas la reproduction du watch-stop applicatif ou du SQL interrompu109.

## Pourquoi une revue distincte ici

Ce test doit tirer une conclusion d'une absence de publication pendant une fenêtre. Sa logique de mesure peut elle-même donner un faux résultat si le worker ne tourne pas, si une opération reste sans résultat ou si la sonde dépasse la fenêtre. Une revue indépendante fresh, ciblée sur ces risques, est donc admise avant de consommer l'unique exécution117. Ce n'est ni une nouvelle campagne globale ni une approbation humaine supplémentaire.

Le briefing et dix entrées locales ont été persistés et copiés sousTI avant admission ; aucun rapport de conclusions de l'auteur n'est fourni. Reviewer strictementRO, sans test, Cargo ou runtime. L'exécution117 reste **non consommée**, publication et pin bloqués. Un PASS du composant ne remplacera pas le rouge de reprise109.

## Annexe

- Source intégrée `services/Telegraph/tests/outbox_runtime_tests.rs`, SHA256 `403e8c90c1b59473c161982bd981a34749797ef09e523423abad92f90124160b`.
- Patch `99ecdf11e7576673e1386da344786e286b37bdd8eb5becefc2bdc17ab4c45c87` ; baseline `008a97375a497ee599bd7297384e2a4d70b28d13f1564237461b2c5f59be5f23`.
- ArchiveTI/.tmp/controlled-claim117-integrated/ :12fichiers source/patch/manifest/bail/fmt et no-run initial/final ; final --no-run status0,12,64s. Les différences110 du fichier vertical chez l'auteur sont préservées, mais ne sont pas intégrées au produitTI, qui reste6ac7.
- Brief canonique `.cursor/review-briefings/20261027-controlled-claim117.md`, copiesTI/.tmp/controlled-claim117-review-inputs/ ; tâche de revue `task_mvqm2ajz_0000_ey1lby`, contextfresh test-reviewer, dépendant de la préparation native terminée.
- Filtre futur exact : `controlled_committed_claim_cancellation_keeps_default_lock_through_25s_observation`, cibleoutbox_runtime_tests seule. Aucun run admis avant verdict et préflight/bail frais.
- Localisation : helpers privés et nouveau test. Impact : preuve prospective du composant, sans contrat transverse modifié. Limites : compilation/inspection, pas preuve comportementale avant exécution ; pas de publication après expiration examinée dans ce budget.
