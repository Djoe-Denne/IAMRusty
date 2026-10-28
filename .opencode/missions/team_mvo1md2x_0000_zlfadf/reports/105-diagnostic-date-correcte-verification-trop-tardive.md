# Le délai est enregistré, mais le test le vérifie trop tard

## Objectif et résultat

La mission doit garantir qu'une notification n'est pas perdue lors d'une panne de messagerie et qu'elle peut être transmise après la reprise, avec les bons droits d'accès. La dernière validation était bloquée sur la date de prochaine tentative.

L'unique diagnostic autorisé est terminé : **un test exécuté, un échec, aucun ignoré**. Il reproduit la même comparaison en échec, sans changer la production, les délais ou les assertions métier. Les nouvelles observations montrent maintenant la cause de cette exécution : la prochaine tentative est bien programmée environ cinq secondes après l'échec, mais le test exige encore qu'elle soit dans le futur après avoir attendu plus de six secondes.

## Ce qui a été observé

- Échec enregistré vers **08:25:58,969 UTC** ; prochaine tentative fixée à **08:26:03,969 UTC** : délai d'environ cinq secondes.
- Lecture précoce vers **08:25:59,044 UTC** : il reste environ **4,93 secondes** avant cette date. La nouvelle tentative n'est donc pas immédiate.
- Comparaison du test vers **08:26:05,028 UTC** : la même date est dépassée d'environ **1,06 seconde**. La ligne enregistrée est identique entre les deux lectures.

Le contrôle arrive après l'attente d'observations de douze secondes déjà présente dans le test. Une date de tentative échue reste légitime : cela autorise une nouvelle tentative, sans imposer qu'elle ait déjà eu lieu à cet instant.

Ce résultat établit une vérification trop tardive pour **ce diagnostic**. Les valeurs exactes de l'exécution antérieure restent absentes et ne sont pas inventées. Aucun défaut de production n'est démontré par cette comparaison. Cela ne prouve pas non plus l'absence de tout défaut : la suite reprise/replay/droits d'accès n'a pas été exécutée dans ce run, arrêté à l'assertion. La mission reste non validée globalement ; publication et référence finale du SDK restent bloquées.

## Nettoyage vérifié indépendamment

Quatre conteneurs créés avec identités tracées : trois absents après teardown ; le quatrième, SMTP, est arrêté avec code de sortie 2 et conservé pour arbitrage, statut ASK au registre. Zéro conteneur possédé actif, zéro Cargo. Deux nouveaux volumes opaques restent conservés ASK. Les dix conteneurs préexistants ont conservé leurs identités et états. Le conteneur étranger actif est préservé ; aucun arrêt global Docker/WSL ni suppression de volume.

## Décision nécessaire maintenant

L'autorisation précédente couvrait seulement les observations et un diagnostic. Elle est consommée. Elle n'autorise pas à corriger le test ou à le relancer.

**Recommandation : autoriser une correction ciblée du contrôle temporel, puis une seule validation verticale.** Vérifier le délai dès la première observation de l'échec, quand l'exigence « date dans le futur » a un sens, tout en conservant les contrôles de persistance et les scénarios complets de reprise, identité stable, replay et autorisation. Ne pas simplement remplacer la comparaison par « date postérieure à la dernière mise à jour », ce qui serait insuffisant. Aucun changement de production, de backoff, de limite de temps ou du sampler ; examen indépendant borné du test pour éviter un affaiblissement de garantie. Réutiliser les preuves HTTP/NoOp encore valides, sans nouvelle infrastructure ni répétition jusqu'au vert.

Ce choix demande une modification de test, sa vérification et une exécution supplémentaire avec les fixtures existantes ; durée non garantie. Un nouvel échec restera un échec à analyser, sans retry automatique. Ce choix ne donne pas une autorisation de publier : la publication restera soumise aux gates existantes et au bilan réel.

**Autre choix : conserver le blocage.** Aucun changement ni nouveau test. Les corrections déjà validées sont conservées, mais la validation complète et la publication restent bloquées par un contrôle temporel dont le défaut vient d'être observé.

## Annexe de preuve

- Campagne : `vertical-diagnostic-55c9f7ba9837411b`, Cargo101, 0 passé/1 échoué/0 ignoré/6 filtrés, 25,05 secondes selon libtest. Une seule cible exacte, aucun NoOp exécuté.
- Source temporaire inchangée après inspection104, SHA `edc39c12ca94852de79168f848dcdc46895c184215c90938f9969ef623b76dec` ; AI1f2 / SDKd60.
- Log : `C:/Users/djden/source/repos/AIForAll/.worktrees/ti/.tmp/vertical-diagnostic-results-55c9f7ba9837411b/notification_created_routing_test.log` ; résumé, inventaires et ledger fixtures archivés à côté, dix copies avec empreintes vérifiées.
- Trace FAILED : updated_at `2026-10-27T08:25:58.969246Z`, next_attempt_at `2026-10-27T08:26:03.969243Z` ; différence exacte 4,999997 secondes, les deux timestamps étant pris séparément. Lecture précoce `08:25:59.043643100Z`, comparaison tardive `08:26:05.027705600Z`.
- Identité SMTP conservée : `3afa81ba1065ca23afffc3b9cbc6d55d35fef1915fa3c72f01d679e8584fe9cf`, Exited(2). Reclassification ASK append-only dans TI/.tmp/docker-resources-20261027.log.
- Localisation : test vertical, comparaison ligne598 instrumentée. Impact : cause temporelle observée ; aucune production modifiée. Limites : scénario interrompu avant reprise ; rouge97 et verdict global non modifiés.
