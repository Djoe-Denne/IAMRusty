# Correctif ciblé : assertions absentes du résumé k6

## Cause démontrée

Le diagnostic parent intégré et rejoué par le lead donne : exception
`System.Management.Automation.RuntimeException`, `PropertyNotFoundStrict`, ligne20
du helper. Le résumé existe,188octets, JSON valide, run_id courant, Chromium,
un cas attendu/démarré/achevé/itération, requests0, mais `checks:{}`.
Les12processus observés du run sont terminés ; tous les jobs active0 et inventaires0→0.

La ligne20 est `$Value.PSObject.Properties.Name` dans `Assert-K6Fields` : le mode
strict PowerShell échoue sur une collection de propriétés vide, avant le rejet
SCHEMA_INVALID prévu. Ne pas transformer ce résumé sans assertions en PASS.

Source officielle épinglée k6v2.3.0 recoupée :
`https://github.com/grafana/k6/blob/v2.3.0/lib/test_state.go#L137-L143`
Le collecteur GroupSummary ignore entièrement les échantillons checks si le tag
group est absent. `internal/cmd/run.go:222–225` utilise ce collecteur pour le
LegacySummary consommé par handleSummary ; `internal/js/summary.go:130–132` expose
root_group. Le scénario désactive group en gardant seulement name/check/scenario.
Ce lien source explique le résumé réellement vide. Pas de défaut de sandbox déduit.

## Delta autorisé — même auteur23/session/tâche/modèle

Compatible avec le contrat révision3 : aucune API, responsabilité, schéma de résumé,
frontière de confiance ou scénario n'est changé. Pas d'amendement structurel requis.

1. Dans le seul appel Windows `k6 run` du mode browser, ajouter l'option fixe
   `--system-tags name,check,scenario,group`. Conserver les trois tags précédents,
   ajouter group pour le rapport d'assertions, sans argument libre ou nouvel export.
   **Ne modifier aucun JS, HTTP, summary.js, pin, workflow ou chemin Linux.**
2. Dans `Assert-K6Fields`, énumérer explicitement les propriétés avant leur nom
   (pipeline ForEach-Object), de sorte que l'objet vide soit rejeté par SCHEMA_INVALID.
   Ne relâcher ni exactitude des champs ni assertions/cardinalités.
3. Régression ciblée : ancien helper contre objet vide échoue avec la mauvaise erreur,
   helper corrigé rend SCHEMA_INVALID ; cas nominal inchangé. Contrôle court du delta,
   pas de suite32redondante, nouvelle fixture Child, vrai k6/Chrome ou autre runtime.

Livrer immédiatement le petit delta et SHA256/Rouge-Vert réellement exécutés.
Le lead rejoue ensuite le même cas réel pour vérifier les trois assertions et le
PASS complet. L'option fixe est une réparation de reporting, pas une nouvelle
protection ou instrumentation de confinement. Les diagnostics privés restent.

## Base et continuité

Base canonique :17a967e + diagnostic parent byte-identique reçu
`result_mvw78r93_0000_klvw5d` (troisfichiers, patch8051octets,
SHA588d3391f8ec344a7743c1429218ee3537dbdde7851d746e592a09d2e5aeee11).
Rapport98 et ledger canonique. Aucun nouveau GO utilisateur, modèle, QA ou revue.
