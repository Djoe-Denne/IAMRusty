# Cause établie : assertions non reportées par k6

## Résultat du diagnostic réel

Le cas navigateur démarre et s'achève ; son résumé courant contient un cas attendu,
démarré et achevé, une itération et zéro requête k6/http. Mais la liste des assertions
est vide. Le parent a correctement refusé un PASS, puis son lecteur PowerShell a
produit une erreur générique sur cet objet vide.

Le diagnostic privé identifie cette erreur : `PropertyNotFoundStrict`, ligne20 du
helper, résumé présent et JSON valide de188octets. Le nouveau run dure2862ms,
parent exit2, k6 natif réussi et cleanup/confidentialitéPASS. Les12processus observés
sont tous terminés, tous les jobs sont à zéro et les inventaires Windows sont0→0.

## Cause et correction retenue

Le code officiel de k6 2.3.0 confirme que le collecteur d'assertions ignore les
échantillons dépourvus du marqueur de groupe. Notre liste restreinte de marqueurs
omettait précisément celui-ci. Le correctif minimal ajoute `group` aux marqueurs
fixes de la seule commande Windows. Aucun scénario, API, schéma de résultat, pin,
sandbox ou parcours HTTP n'est modifié.

Le lecteur PowerShell est également corrigé pour rejeter explicitement un résumé
vide avec SCHEMA_INVALID. Les critères ne sont pas réduits : les trois assertions
nommées devront encore être présentes, chacune exécutée une fois et réussie.

Le même auteur livre ce petit delta et son test ciblé. Le lead relance ensuite le
cas réel ; aucune nouvelle revue, protection ou décision utilisateur n'est demandée.

## Preuves techniques

- Diagnostic source reçu `result_mvw78r93_0000_klvw5d` :3fichiers intégrés avec SHA256
  identiques ; test inerte1/exit0/zéroskip/0.571s, aucun nouveau Child ou vrai outil.
- `git diff --check`exit0 avant le run ; preuve privée
  `.tmp/k6-browser-r3-20261031-retry2-lease.json` et fichiers parent-error/parent-summary.
- [k6v2.3.0 GroupSummary](https://github.com/grafana/k6/blob/v2.3.0/lib/test_state.go#L137-L143),
  [LegacySummary](https://github.com/grafana/k6/blob/v2.3.0/internal/cmd/run.go#L222-L225),
  [root_group](https://github.com/grafana/k6/blob/v2.3.0/internal/js/summary.go#L130-L132),
  sources épinglées réellement relues. Le schéma n'a pas changé ; aucune réécriture
  de summary.js ni nouvelle métrique personnalisée n'est retenue.
- L'hypothèse d'une absence du fichier résumé est écartée : il existe et son run_id
  correspond au run courant. La cause n'est ni un crash Chrome ni une attestation.
- Pas encore de PASS navigateur complet. HTTP/selftest, WSL et Linux restent
  INCONCLUSIVE ; aucun runtime tiers, déploiement ou appel applicatif.
