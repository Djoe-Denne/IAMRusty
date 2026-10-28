# Rapport 74 — plan correctif validé, admission native à reprendre

## Plan reçu du même auteur

pool-9 reprend sa session/historique et remet le plan lecture seule. Lead valide la direction dans le contrat existant : diagnostic sticky séparé de la complétude ; driver privé possédé créé au premier stop, budget watch monotone ; un seul consommateur du JoinSet ; observation du coordinateur transférée sans perte des résultats déjà acquis ; force indépendante du futur join et du coordinateur ; driver lui-même observé/joint, guard d'annulation et Drop sans await. Aucun changement API/protocole/manifest/adoption AI prévu, tests426 intacts.

Après deadline, la récolte peut continuer sans transformer une erreur déjà rendue en succès ; pas nouveau délai de réussite. Raccourcir une échéance pendant une attente doit réveiller le pilote et conserver les minima. Ces obligations ne constituent ni nouveau backend/harness ni diminution du contrat ; détails privés laissés à l'auteur. Tests T1/T2 déjà rouges, pas à redécouvrir.

## État natif et continuité

Le contrôle d'admission constate que pool-9 est encore marqué **shutdown/preserved** dans l'équipe malgré la capacité de sa session à recevoir/rendre le plan. Ne pas confondre reprise d'une conversation et affectation d'une nouvelle exécution production. Une admission native explicite est nécessaire avant GOcode.

Le lead privilégie l'historique de l'auteur via continuité native, sans fork de lead ou orchestrateur intermédiaire. Si le membre scellé ne peut reprendre l'affectation, utiliser un fork de son contexte avec ownership SDK exclusif dans son arbre préservé (source426 déjà prête), pas une redécouverte fresh silencieuse ou un changement de modèle. Aucun écrivain concurrent dans cet arbre ; SDK utilisateur frère intouchable. Le résultat réel de cette admission sera consigné avant mutation autorisée.

## Vérification bornée

Correctif SDK privé seul, base426 ; aucun test modifié par productionwriter. Après compilation/fmt, exécuter T1/T2 puis seulement les régressions existantes touchées de join/Drop/panic/force/grâce. UniqueCargoWindows24 et logs/provenance, pas Docker/globalFGA/SQS ni push/pin. Revue ciblée du même reviewer après diff intégré ; T4 finale reste gate avant clôture. Aucun correctif ou vert acquis à ce checkpoint.
