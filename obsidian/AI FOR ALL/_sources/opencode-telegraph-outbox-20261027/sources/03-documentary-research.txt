# Recherche web : conserver la lease, simplifier la preuve

## Conclusion pratique

La recherche ne révèle pas de correctif de production indispensable déjà démontré. **La solution minimale sûre est d'abord de tester la reprise à partir de l'éligibilité stockée, et non d'exiger une publication avant l'expiration d'un verrou.** Une intention protégée par une lease SQL de 30 secondes peut normalement attendre cette échéance après l'annulation de son propriétaire.

Une amélioration de production raisonnable existe ensuite : un arrêt coopératif qui cesse les nouveaux claims et laisse l'opération active finir dans la grâce existante. Elle peut réduire cette fenêtre lors d'un arrêt normal, mais ne remplace pas la récupération après crash ou annulation forcée. Sa nécessité dans ce lot n'est pas établie.

La question de remplacement127 reste **de côté conformément à la demande humaine128**. Cette synthèse n'est pas un GO : aucun test/build, patch applicatif ou changement de garantie n'a été exécuté pour la recherche, et aucune relance ne sera faite sans décision distincte.

## Pourquoi nous tournions autour du même point

Trois temporalités différentes étaient mêlées : arrêt des tâches locales en 6 secondes, expiration d'un claimSQL à 30 secondes, puis contrôle de publication du test après 25 secondes depuis reconstruction. L'arrêt local réussi ne signifie ni rollback d'une commande déjà envoyée à PostgreSQL, ni libération de la réservation durable. Le test fixe de 25 secondes ne peut donc pas couvrir tous les états permis par la lease de 30 secondes.

Il faut aussi séparer deux incidents de test du problème distribué : la comparaison d'une date future après un sampler de 12 secondes, corrigée dans le test, puis l'identifiant de campagne de 41 caractères rejeté par un maximum de 40. Le second empêche encore toute mesure post-expiration dans122 ; ce n'est pas un nouvel échec métier.

Le témoin117 prouve la conservation du verrou et l'absence de claim pendant sa fenêtre contrôlée. Il ne prouve pas le sort historique109 ou l'envoi après expiration. Le vert110 n'avait pas rencontré ce claim retenu. Les sources web confortent cette distinction, sans reconstruire les valeurs qui manquent.

## Ce que les sources établissent

**Outbox et doublons.** AWS recommande d'enregistrer la modification métier et l'intention d'événement dans la même transaction, puis de publier depuis l'outbox. Le relay peut envoyer des doublons ; le consommateur doit être idempotent [1–2]. Cela ne rend pas SQL et SQS atomiques ensemble. Avec un budget fini d'essais, il n'existe pas non plus de promesse de livraison inconditionnelle malgré une panne permanente.

**Annulation locale et effet distant.** Tokio indique que `select!` annule les autres branches quand l'une termine ; sa définition de cancellation safety porte sur l'innocuité d'abandonner et recréer une future [4]. PostgreSQL documente qu'une annulation reçue après la fin de la requête peut être sans effet [3]. Rien n'autorise à assimiler un abandon local à un rollback distant. La documentationSQLx distingue la transaction encore en cours et la fermeture de connexion ; elle ne décrit pas assez précisément le drop de notre futur de requête et le recyclage du pool pour en déduire sa causalité exacte [5].

**Arrêt propre.** Tokio distingue demander aux tâches de s'arrêter et attendre leur terminaison [4]. Le dépôt ajoute son propre budget : grâce jusqu'à5s, jointures jusqu'à6s absolues, stops inclus. Un drain borné ne peut pas garantir qu'un réseau arbitrairement lent finira ; la lease et la reprise restent nécessaires après la borne ou un crash.

**SQS n'est pas la leaseSQL.** Le VisibilityTimeout masque un message déjà reçu par un consommateur ; il ne pilote pas locked_until dans notre table [6]. Sa calibrationIAM à5s ne justifie donc pas de réduire la leaseSQL à5s. La déduplicationFIFO décrite par le SDK est bornée à5minutes et ne constitue pas une garantie exactly-once de la transaction métier.

**Tokens et conditions.** Des transitionsSQL conditionnées au propriétaire/génération peuvent empêcher un ancien worker d'écraser un nouveau claim. Elles ne forcent pas SQS à rejeter un ancien envoi ; un vrai fencing exige la participation du destinataire [7]. L'idempotence et la convergence restent des obligations aux frontières concernées, pas une garantie globale ajoutée à tous les services par cette recherche.

## Trois voies, par priorité

### 1. Aligner le protocole de preuve — minimum recommandé

Conserver le produit et les leases actuelles. Distinguer conservation avant l'éligibilité, reclaim après l'éligibilité, succès du vrai transport puis étatpublished et message réellement reçu, avec la même identité/payload/DB.

Le budget fixe déjà préparé pour le composant — t0 à l'entrée de reconstruction, résiduelR mesuré une fois, D=t0+R+25s avecR<=30s doncD<=t0+55s — est cohérent avec cette garantie **si** l'état, les horloges, le budget d'essais et le transport sont vérifiés. Ce calcul est notre protocoleQA, pas une formule prescrite parAWS ou un SLA produit. Ne jamais repousser D à chaque retry ou nouvelle lecture.

Une lecture n'est pas une barrière contre une ancienne commandeSQL déjà émise. Si l'état change après la photographie initiale, le constater et conserver le diagnostic/échec plutôt que recalculer indéfiniment la deadline. Le cas contrôlé après claim terminé isole volontairement ce risque ; il ne remplace pas la verticale.

**Conséquence :** pas de nouvelle architecture ou infrastructure, et pas d'augmentation aveugle d'un timeout pour obtenir un vert. La preuve après expiration reste à acquérir. La correction de nom du lanceur est distincte et ne nécessite aucun changement du SDK ou du test revu445f.

### 2. Arrêt coopératif avec drain borné — candidat facultatif

À l'arrêt normal, cesser les nouveaux cycles/claims, laisser l'unité active claim/envoi/marquage atteindre son résultat pendant la grâce restante, interrompre l'idle immédiatement, puis garder l'annulation forcée et la jointure à la deadline existante.

Le premier endroit à examiner serait le wrapper externe qui abandonne actuellement dispatcher.start dès le signal, plutôt qu'un nouveau coordinateur. Mais supprimer simplement son select n'est pas une preuve : le dispatcher réclame des batches de25 avant des publications séquentielles, dort entre cycles et remet son indicateurstop àfalse au start. Il faut spécifier l'unité drainée et les courses start/stop, sans dépasser5s/6s.

**Conséquence :** peut rendre l'arrêt normal moins perturbant, pas supprimer l'ambiguïté après crash ou deadline. Le protocole de reprise reste nécessaire. Toute modification de rustycog-outbox partagé dépasse l'exception rustycog-events déjà autorisée ; aucun patch de cette voie n'est lancé ou approuvé ici.

### 3. Restitution immédiate du claim — ne pas choisir par défaut

Une libération anticipée ne serait sûre que si le claim est encore possédé et qu'aucun SQL/envoi actif ou ambigu ne peut agir ensuite. Sinon il faut concevoir acquisition et transitions conditionnelles, contrôle des lignes affectées, identité/budget et comportement de l'ancien propriétaire.

Les mises à jour parID actuelles et l'hypothèse d'un seul dispatcher ne justifient pas un simple unlock global. Une ancienne publication peut réussir à distance pendant une nouvelle tentative. **Conséquence :** nouvelle exigence persistée/concurrente, risques de doublons et mandat architectural distinct. Pas de besoin établi pour l'introduire maintenant. CDC/Debezium ou nouvelle infrastructure seraient également disproportionnés pour le problème démontré.

## Ce qu'il ne faut pas faire

- Réduire la leaseSQL à5s par analogie avec la visibilitéSQS ou pour rendre le test vert.
- Effacer tous les locks au shutdown, remettre les tentatives àzéro ou rendre les retries infinis.
- Supposer «future abandonnée = transaction annulée» ou «aucun message reçu = aucun envoi accepté».
- Présenter le drain comme une solution de crash, un tokenSQL comme du fencingSQS ou FIFO comme de l'exactly-once métier.
- Transformer un PASS de composant en validation des droits/replay/wholeapp, des AC1–AC7 ou en autorisation de publier.

## Recommandation de pilotage, sans nouvelle exécution

Ne pas ouvrir une nouvelle boucle d'architecture ou de tests exploratoires. La recherche confirme un pattern connu et une incompatibilité précise du premier oracle temporel. Garder les preuves117/109/110 séparées et la preuve après expiration explicitement absente. La prochaine décision utile, quand l'utilisateur souhaitera reprendre, concerne le composant déjà revu avec le paramètre de lancement valide — pas une nouvelle réécriture de production pour contourner la lease.

L'arrêt coopératif reste une amélioration à évaluer seulement si une exigence produit ou une mesure la justifie. Les inconnues restent : état exact109, livraison après expiration, issue de SQL/envois annulés, durée d'une unité/batch face à la grâce et convergence d'autres effets aval. Pas de nouveau défaut produit prouvé par ces inconnues.

## Bibliographie consultée le27/10/2026

1. [AWS — Transactional outbox](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html), «Issues and considerations / Duplicate messages», table relationnelle et atomicité locale.
2. [AWS Builders' Library — Making retries safe with idempotent APIs](https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/), requêtes tardives et identité de demande.
3. [PostgreSQL18 — Canceling Requests](https://www.postgresql.org/docs/18/protocol-flow.html#PROTOCOL-FLOW-CANCELING-REQUESTS) et [Transactions](https://www.postgresql.org/docs/18/tutorial-transactions.html). RéférencesPG18, pas validation de la version effectivement exécutée du serveur ni du mécanisme précis de poolSQLx.
4. [Tokio — Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown), demande/attente ; [source primaire versionnée select!1.52.3](https://raw.githubusercontent.com/tokio-rs/tokio/tokio-1.52.3/tokio/src/macros/select.rs), introduction/cancellation safety. La versionHTMLdocs.rs de select a échoué à la décompression chez l'agent ; source primaire versionnée utilisée à la place.
5. SQLx0.8.6 : [Connection::close](https://docs.rs/sqlx/0.8.6/sqlx/trait.Connection.html#tymethod.close), [Transaction/Drop](https://docs.rs/sqlx/0.8.6/sqlx/struct.Transaction.html). Ne pas extrapoler ces contrats à tous les drops de futur de requête.
6. [AWS — SQS visibility timeout](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-visibility-timeout.html) ; [aws-sdk-sqs1.100.0 — message_deduplication_id](https://docs.rs/aws-sdk-sqs/1.100.0/aws_sdk_sqs/operation/send_message/builders/struct.SendMessageFluentBuilder.html#method.message_deduplication_id).
7. [PostgreSQL18 — Read Committed/WHERE](https://www.postgresql.org/docs/18/transaction-iso.html#XACT-READ-COMMITTED) ; [Martin Kleppmann,08/02/2016 — fencing tokens](https://martin.kleppmann.com/2016/02/08/how-to-do-distributed-locking.html), référenceconceptuelle, pas recommandation deRedis.

## Provenance et vérification

Agent documentairepool20/general fresh, tâchetask_mvqsmvbv_000i_xksswr, strictRO : sources web effectivement consultées et clauses/corps utiles des neuf inputs locaux lus. Aucun code/test/canon/docproduit édité, Cargo/Docker/SQL/build/pub/pin ou ressource créée, aucune délégation. Le niveau d'effort n'est pas attesté dans le résultat ; ne pas le présenter comme une sélectionhigh confirmée.

Le lead a recoupé indépendamment quatre sources primaires parContextMode fetch/index, toutes accessibles : AWSoutbox, TokioShutdown, PG18Cancel, sourceTokioselect1.52.3. Passages vérifiés : doublons/idempotence, demande d'arrêt distincte de l'attente, annulation tardive sans effet, branchesselect annulées/définition de cancellation safety. Les autres références sont celles effectivement lues par l'agent ; pas toutes recoupées une seconde fois.

Sources de code ciblées : dispatcher37–45/94–99/121–187/197–255, setup-app239–249/259–276, setup-runtime65–79/115–160, contrat73–77/178–180/194–196. Pas de conclusion d'absence à partir de l'index canonique horsTI. VersionsCargo.lock luesstatique : Tokio1.52.3/SQLx0.8.6/SeaORM1.1.20/aws-sdk-sqs1.100.0 ; aucune garantie supplémentaire SeaORM d'annulation distante établie.

Localisation : protocoleQA de reprise et frontière arrêt/lease/publish. Impact : recommandationdocumentaire priorisée, pas implémentation. Limites : ni cause historique109 ni preuve après expiration, ni nouveau run ; pause128 et gatespub/pin maintenues. Tâcherecherche terminée, résultats/URLs persistés, question127 non reposée.
