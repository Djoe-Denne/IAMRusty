# Synchronisation limitée à l'organisation autorisée

Le correctif empêchant de créer un job sur le lien d'une autre organisation est
intégré. L'organisation autorisée est maintenant comparée à celle du lien avant
toute création ou sauvegarde du job, dans les deux chemins de persistance.
Le refus ne révèle pas si le lien étranger existe ; le cas légitime reste accepté.

## Preuves reçues et vérifiées

Six tests distincts réussissent. Retirer temporairement la seule garde fait
échouer le test inter-organisations ; la restauration exacte du code ramène les
tests au vert. Ce test de mutation démontre que l'oracle détecte la protection
manquante, sans prétendre avoir exécuté l'ancienne API historique.

Les quatre fichiers intégrés correspondent exactement à la livraison auteur,
et le formatage global réussit au commit canonique. La revue de sécurité initiale
relit ce correctif. Cela ne remplace pas les tests HTTP/PostgreSQL/OpenFGA réels.

## Suite autorisée

Le dernier auteur termine les tests de pagination, snapshot de suppression,
migration et délai cumulé du signataire. Son créneau exclusif permet ensuite de
compiler le binaire Hive ciblé sans l'exécuter et de tester le délai remote sur
sockets locales. Les tests PostgreSQL restent réservés à la campagne finale,
après intégration et inventaire des ressources.

Aucun Docker, Kind ou navigateur de mission n'est démarré. Aucun nouveau GO
utilisateur n'est demandé et aucune qualification globale n'est annoncée.

## Annexe technique

- Livraison455306df77f6b7b33dee60199a1a4ac3e6da9acf, intégration4b2ceb51d7f4d3545d7b361bf1f2cbcd5f9a13e1 ; quatre fichiers Hive, pas de contrat HTTP/persisté ou nouveau port.
- Application2PASS (3 autres filtrés), domaine4PASS (13 autres filtrés),0ignored ; mutation1PASS/1FAILattendu, restauration2PASS. Ne pas additionner les exécutions superposées.
- Sept logs exacts archivés sous `.tmp/auth-s1-pool17-proof-455306d`, SHA256 copie/source identiques, traces natives recoupées via Context Mode.
- Merge natif réussi ; branche conservée, cleanupworktree pending. Hookfmt normal du commit canon réussi, configurationétrangèreSDK/Obsidian préservée.
- Recheck S-1 demandé à la même session sécurité ; S-2 déjà clos statiquement. QA T1/T2 reste bloquante tant que les preuves réelles ne sont pas acquises.

Actualisation : le recheck indépendant clôt **S-1**, sans nouveau finding sur le
correctif ni le helper privé de membres. Les deux défauts de sécurité signalés
sont clos au niveau source. [Briefing de relecture](../../../../.cursor/review-briefings/20261028T2255Z-security-sync-boundary-recheck-4b2ceb5.md).
Les tests réels et la qualification d'ensemble restent ouverts.
