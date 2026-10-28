# Le témoin prouve un verrou maintenu après annulation

## Résultat et sens du succès

Le seul test contrôlé autorisé117 est terminé : **1 passé, 0 échoué, 0 ignoré**. Son succès signifie que le comportement prédit a été mesuré, **pas qu'une notification a été publiée dans les25secondes**.

Une notification réelle a été prise en charge enSQL, puis son publisher suspendu et son propriétaire annulé et joint. La ligne est restée publishing, avec le même identifiant d'événement, une tentative et le verrou de l'ancien worker. La nouvelle instance a réellement effectué **13 sélections SQL** :13 cycles commencés,13 terminés,0claim. Les13 observations de ligne ont conservé identité, état, tentatives et verrou. Aucun message observé lors de la sonde SQS ponctuelle ; ce n'est pas une garantie universelle sur le broker.

La fenêtre de25secondes et le cleanup se terminent alors qu'il reste environ4,984secondes avant l'expiration du verrou30s. Les propriétaires suspendus et reconstruits sont détruits et joints. L'absence de reprise ne vient donc pas d'un nouveau dispatcher inactif dans ce témoin : la ligne n'est simplement pas encore éligible.

## Ce qui est désormais établi, et ce qui ne l'est pas

Le mécanisme prospectif est prouvé : **annuler après un claim terminé peut laisser une intention durablement verrouillée, non reprise pendant une observation de25s**. Cela explique une classe de situations compatible avec l'échec109 ; les valeurs historiques manquantes109 restent inconnues. Le témoin n'exerce pas l'annulation pendantSQL ou l'application entière et n'observe pas l'envoi après expiration. Aucune perte, violation d'identité/droits ou tâche locale orpheline n'est démontrée.

Le contrat prévoit une reprise après expiration du verrou, sans engagement produit de25s. Il faut maintenant distinguer la preuve de cette garantie d'un éventuel changement du comportement d'arrêt. **Ni correction de production, ni augmentation tacite de délai, ni publication n'est autorisée par ce résultat.** Le rouge109 et le vert110 restent séparés ; le témoin117 ne les remplace pas.

## Nettoyage vérifié indépendamment

Quatre conteneurs créés : trois absents après teardown, SMTP arrêté Exited(2) conservé ASK. Deux nouveaux volumes opaques conservés ASK. Zéro conteneur possédé actif et zéro Cargo. Les treize conteneurs préexistants gardent leurs identités et états. Workload étranger actif préservé ; aucun arrêt global Docker/WSL ni suppression de volume. Vérification persistée à côté des preuves ; reclassificationSMTP append-only au registre.

## Étape suivante, sans nouvelle exécution

Le lead demande une lecture architecturale bornée du canon et des preuves, avant de proposer la suite à l'utilisateur :

- La garantie voulue permet-elle de conserver le verrou puis de prouver la reprise après expiration, avec un protocole de test explicitement borné ?
- Une exigence supplémentaire de reprise immédiate à l'arrêt impliquerait-elle au contraire un changement de contrat ou de production, avec risques de doublons et nouvelle autorisation ?

Cette consultation est lecture seule, pas une décision de réduire les garanties ou de modifier un test. Toutes les exécutions autorisées sont consommées. La prochaine correction ou preuve après expiration exige un arbitrage explicite et borné, pas un nouveau run automatique.

### Consultation admise et photographie actualisée

La tâche `task_mvqmhthd_0004_sscmxe` est admise pour architecte en contextefresh, strictementRO, sans score ou modèle imposé. Seize entrées exactes ont été copiées et leurs empreintes vérifiées sousTI/.tmp/architecture-recovery120-inputs/ avant admission : ADR/contrats avec statuts, sources exécutées et preuves117. L'avis doit citer la cible canonique et séparer une éventuelle adaptation de protocoleQA d'un changement de garantie produit ; aucune ADR ou source ne sera écrite par cette consultation.

Les deux photographies documentaires sont actualisées avec le résultat117 et ses limites, sans changement d'AC ou statutADR. La preuve après expiration reste explicitement absente, la publication bloquée et le rouge109 conservé.

## Annexe

- Campagne `controlled-claim117-695e9e1ccbdd409c`, Cargo0/launcher0,1PASS/0FAIL/0ignored/5filtered,35,65s libtest. Filtre exact117, aucun autre test.
- AI1f2/SDKd60 local/index129, test intégréSHA403e8c90, patch99ecdf11, revue indépendante119/brief20261027-tests-controlled-claim117 inchangés.
- Claim updated_at11:45:01.293208Z, lock_until11:45:31.293208Z ; annulation11:45:01.303702100Z, snapshot propriétaire joint11:45:01.306156200Z ; même row/event/aggregate/ancienlocked_by.
- Début fenêtre11:45:01.308255Z, deadline11:45:26.308255Z ; outcome11:45:26.309550700Z :25001ms,cycles13/13/13,claims0,actual_claims0,identité/verrou inchangés,new_owner_destroyed1,broker_emptytrue.
- Preuves privéesTI/.tmp/controlled-claim117-695e9e1ccbdd409c/{summary.json,outbox_runtime_tests.log,before.jsonl,after.jsonl,lead-cleanup-verification.json,fixture-*.jsonl}, sources etcompilation séparées danscontrolled-claim117-integrated.
- Localisation : test composant et règles d'éligibilité du dispatcher. Impact : séparation démontrée entre jointure locale et expiration d'un verrou durable. Limites : pas causalité historique109, pas envoi après expiration ou quitus global AC1–7/ADR/pin/publication.
