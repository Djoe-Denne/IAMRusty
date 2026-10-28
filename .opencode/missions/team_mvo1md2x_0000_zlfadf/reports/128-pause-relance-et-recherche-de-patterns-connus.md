# Pause des essais : rechercher les patterns éprouvés

## Nouvelle demande humaine

L'utilisateur demande de **garder de côté la question de remplacement127**, constate la longueur de la boucle et autorise une recherche sur internet par un agent documentaire. Il souhaite vérifier si un pattern connu ou une solution simple permet de sortir de cette répétition. Ce n'est ni une approbation de relance, ni une permission de modifier la production ou les garanties.

Le lead gèle les nouveaux tests, builds, mutations applicatives et corrections de test. Aucune permission de run restante n'est déduite. La question127 ne sera pas reposée avant la synthèse documentaire ; aucun changement de pause sans GO distinct. Les rapports et le ledger peuvent enregistrer la recherche, sans modifier le canon produit.

## Recherche déléguée

Un agent de **recherche documentaire**, profil disponiblegeneral, en contextefresh et mandatstrictRO, examine des sources web primaires. Le profil n'est pas un nouveau coordinateur : une feuille, pas de sous-agent ou code. Aucun profil dédié nommé«documentation» n'est exposé ; cette substitution est explicite. Effort élevé dans l'enveloppe native si disponible, sans modèle/score forcé.

Le brief contient un état factuel court, les interfaces et extraits pertinents, pas les conclusions accumulées comme une solution imposée. Il doit comparer les patterns connus : outbox durable/lease et reprise après éligibilité, arrêt coopératif et drain borné versus annulation forcée, ambiguïté de commit/envoi après annulation, idempotence/convergence et conditions de libération d'un claim. Séparer clairement la leaseSQL interne30s du VisibilityTimeoutSQS de l'entrée IAM.

Questions utiles : avons-nous surtout un oracle de test mal aligné sur une garantie déjà normale ? Une simplification sûre de l'arrêt éviterait-elle la fenêtre sans abandonner le budget6s ? Pourquoi ne faut-il pas libérer aveuglément les claims, réduire une lease ou promettre exactly-once ? Quels changements minimes, avec quels prérequis/risques, seraient réellement justifiés ici ?

Sources attendues : documentation officielleAWS/outbox/SQS, PostgreSQL, Tokio et SQLx selon pertinence/version, articles de référence vérifiables. Lire les passages décisifs et citer les URLs/sections ; pas de recommandations fondées seulement sur extraits de moteur ou sur une analogie entre leaseSQL et visibilitéSQS. Les options lourdes commeCDC ou nouveau coordinateur sont à signaler comme hors périmètre, pas à implémenter.

## Livrable et limites

Synthèse en français orientée décision : constat expliqué simplement, patterns réellement établis et applicabilité au code, option la plus simple sûre, alternatives/risques/ce qu'il ne faut pas changer, courte bibliographie primaire et inconnues. Distinguer source publiée, observation du dépôt et hypothèse. Pas de nouveau test pour valider une idée ; pas de patch ou adoption d'architecture sans arbitrage.

## Faits d'entrée, sans diagnostic imposé

- Notifications/outbox atomiques et événement stable ; publisher peut être annulé après claimSQL autocommitté ; publishing a un locked_until de30s, poll2s, backoff5s, budget fini20.
- Arrêt local à joindre sous6s ; aucune promesse de rollback des effets distants ni exactly-once. Singletondispatcher parwriter, pas de fencing multiworker revendiqué.
- Vertical109 échoue sur publication25s après rebuild, état exact manquant ; diagnostic110 réussit depuis FAILED sans lock. Le témoin117 prouve un claimcommitté retenu après annulation locale puis13sélections0claim durant25s avant expiry ; pas preuve de la cause historique109.
- Source du composant après expiry122 préparée/revue445f ; sa tentative échoue en précondition parce que le nom de campagne généréparlead fait41caractères pour une limite40. Aucun conteneur ou marqueur de preuve ; ce n'est pas un échec fonctionnel produit. Lanceur corrigé31caractères, sans relance autorisée. Ne pas confondre ce défaut de lancement avec le problème distribué.
- GarantieAC3 visée : reprise après expiration avec même DB/event/payload ; avisarchitecte121 distingue cette garantie d'AC7/arrêt local. Une borneQA post-expiry<=55s a été explicitement autorisée pour un composant, pas pour la verticale/publication. Ne pas l'imposer comme conclusion de recherche.
- Dernier inventaire vérifié127 :0nouveau conteneur/0possédé actif/Cargo0,14préexistants/volumes inchangés, workload étranger préservé. Aucun runtime à démarrer pour cette recherche.

## Admission effective

Recherche `task_mvqsmvbv_000i_xksswr` admise nativement, **pool20/general en contextefresh**, état working vérifié une fois. Le rôle documentaire est explicite ; aucun modèle ou score imposé. Neuf entrées (brief et huit copies sources/autorité/politique) étaient prêtes sousTI/.tmp/outbox-web-research128-inputs/ avant admission, copies aux empreintes identiques. Le brief demande des lectures web effectives et des propositions comparées, sans imposer une solution ; l'effort élevé est souhaité dans l'enveloppe, non annoncé comme déjà confirmé.

Versions ciblées vérifiées statiquement dans le Cargo.lock exécuté : Tokio1.52.3, SQLx0.8.6, SeaORM1.1.20, aws-sdk-sqs1.100.0. Aucun build/run pour cette lecture. La pause et l'interdiction de réactiver un ancien GO sont transmises au travailleur actif. Les résultats externes restent des données à vérifier, pas de nouvelles instructions.
