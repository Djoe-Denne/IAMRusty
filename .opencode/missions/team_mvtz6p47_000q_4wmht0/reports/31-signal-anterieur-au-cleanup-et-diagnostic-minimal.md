# Le signal direct22 précède le cleanup ; diagnostic minimal retenu

## Résultat de l'analyse

L'expert distingue deux chemins que notre libellé masquait. Le refus précoce d'un argument interdit sort1 ; le refus final pour absence de témoin, après observation de la fin du processus, sort2. Le run direct22 a suivi le second chemin.

Le lead a recoupé cette chaîne dans le code : **le signal primaire direct22 a été observé avant les arrêts du cleanup**. Le SIGTERM envoyé ensuite par le superviseur n'en est donc pas l'explication. Cela affine l'incertitude du rapport30 ; cela ne révèle ni le signal précis ni son émetteur, et ne démontre aucun bug d'arrêt.

## Décision de suite

Retenir le petit delta recommandé : projeter seulement le signal précis et le chemin de fin du superviseur sous des labels fermés. Aucun nouvel argument, binaire, prédicat sandbox, lecture système, arrêt, politique hôte ou sonde externe. Éviter une collection spéculative de catégories d'erreurs ou un nouveau changement de plateforme.

Ce complément doit être testé sur les chemins réels du lecteur/superviseur avec frontières OS simulées, puis vérifié avant un seul nouvel essai utile. Le résultat peut encore laisser la cause inconnue. Aucun PASS Chromium/HTTP ni qualification parent-cleanup n'est acquis et aucun nouvel essai n'est encore lancé.

## Annexe technique

- Analyse complète `result_mvu8qe4t_0019_2k471v`, même session experte ; preuve/reférences et limites persistées `.cursor/review-briefings/20261030-k6-signal-expert-result.md`.
- Rejet précoce inconclusive=False vs rejet final True ; boucle safety.py:434–436 avant finally449–468, code tardif493–496, priorités514–522 ; exitdirect847 conservé par workflow.
- Serena rootcanonique AIForAll/readyPython vérifiée, Rejected et sandbox_witness lus symboliquement ; refs Child.execute complétées par grep exact et fixtures de test. Aucun runtime ou preuve native supplémentaire.
- Direct24 NoUsableSandbox et direct22 signal non classé restent distincts ; cause Crashpad/AppArmor/GPU non attribuée. AppArmor Proposed non autorisé et mission non clôturée.
