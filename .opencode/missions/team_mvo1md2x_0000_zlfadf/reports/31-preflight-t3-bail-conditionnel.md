# Rapport 31 — garde SMTP recoupé, bail IT conditionnel

## Vérifié par le lead

Sources réelles du worktree T3, non indexées par le MCP canonique : fixture SMTP22–32 conserve le singleton en présence d'Arc consommateurs ;52–69 témoin de destruction ;95–99 stop/rm via Owned, erreurs propagées ;106 lease conservé par TestSmtp ;190–204 aucun fallback par nom, refus explicite de bail actif, barrière des créations. Framework fixture_runtime222–253 et718–820 : mécanismes existants, identité exacte vérifiée avant rm. Cette inspection ne remplace pas la revue indépendante finale de T3.

Log `smtp-ownership-unit.log` lu : un test exécuté/passé, zéro ignoré, un hors filtre. Inventaire `preflight-containers.log` lu : unique container40201bda8bb2d1eeaa51287be96fc3448752c219dba74791fa326c5071705f29, running/non possédé. Auteur annonce daemon-time2026-10-25T19:08:19.140882097Z,28images dont quatre requises en cache et108volumes non adoptés ; ces deux totaux n'ont pas été recomptés par le lead.

## Bail borné et gate maintenue

Bail parent enregistré pour les seules fixtures que créera le harness des quatre cibles contractuelles T3. IDs à création, registre contemporain, cleanup en fin et en échec, zéro possédé running, pas de rétention pour la suite. Node et volumes inconnus/ask intouchés, aucun nettoyage global. Le bail n'autorise pas de suppression arbitraire.

**Pas de GO run** tant que le wrapper opérationnel n'est pas récupéré/inspecté. Source originale T2 demandée à sa session, aucun fichier wrapper trouvé dans l'archive ou les deux dossiers de runs. Inventaire à recouper s'il change avant départ. Get-or-create SMTP échoue intentionnellement si consommateur encore actif : préserver cet oracle, pas de bypass de lease pour faire passer la suite.

Aucune nouvelle compilation ou campagne lead, aucun runtime créé par le lead. AC1–AC7 globalement non validés ; preuve verticale toujours attendue.
