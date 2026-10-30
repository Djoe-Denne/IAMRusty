# Isolation Windows : limite de preuve confirmée

Le but reste de vérifier les outils et l'unique cas navigateur autorisé. L'avis de
l'architecte confirme que les observations prévues ne suffisent pas à qualifier
l'isolation effective du processus qui exécute la page. Observer son jeton principal
restreint ne prouve pas que tous les droits plus larges du démarrage ont été retirés.

La décision est donc de conserver **Chromium INCONCLUSIVE**, même si la version et
le JavaScript réussissent. Aucun nouveau dispositif d'observation, droit privilégié
ou contournement n'est ajouté pour obtenir un résultat vert.

Ce manque de preuve n'est ni une désactivation du sandbox ni une évasion démontrée.
Après correction vérifiée du risque de nettoyage et du contrôle des bibliothèques
Chrome, puis relecture ciblée, le seul cas about:blank pourra être exécuté dans
son périmètre existant, avec ce verdict non qualifiant. Il reste gelé jusque-là.

Les corrections se poursuivent. Aucun k6 ou Chrome Windows réel, GET applicatif,
WSL ou Linux n'a été exécuté par cet avis. Les résultats futurs des versions,
checks et nettoyages resteront séparés, sans PASS global inventé.

## Références

- [Avis et disposition complète](../../../../.cursor/review-briefings/20261030-k6-windows-effective-sandbox-result.md),
  retour architecte result_mvvd4ub4_001d_dm7qv1, sourcesChromium156 et Microsoft.
- [Protections encore à corriger](84-protections-a-corriger-avant-le-premier-navigateur-windows.md).
- Aucun changement de la politique native ou de son niveau QA ; pas de nouvelle
  approbation de jalon demandée et aucune levée anticipée du gel.
