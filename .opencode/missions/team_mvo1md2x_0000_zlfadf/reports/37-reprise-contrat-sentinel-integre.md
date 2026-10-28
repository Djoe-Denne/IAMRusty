# Rapport 37 — reprise après redémarrage, contrat Sentinel intégré

## Continuité vérifiée

Pas de test rejoué, pas de création de runtime pour la reprise. État de l'équipe contrôlé une fois : mission existante long_run/running, t3-runtime idle et propriétaire de T3, autres entrées historiques préservées. L'ancien appel architecte avait été interrompu sans résultat et sans diff ; contrôle du checkout lead propre avant reprise documentaire.

Le lead a réutilisé l'autorisation humaine du rapport36, sans la redemander. Ancienne session native T1 non joignable (message queued non spawned) : substitution explicite par architecte, session `ses_e87561177fferReewD4GYQX2c3`, feuille documentaire sans Cargo/runtime. Amendement §6 rédigé par architecte : exception classifier+tests et négatifs fail-closed, pas extension générale. Référence du run corrigée par le lead au nom exact `t3-routing-2ebb73f602484bdf`.

## Contrat intégré et remis

Commit `09a59ee05fec5c45110c461fe24e2954a33d3122`, un seul fichier docs/contracts/telegraph-notification-outbox.md, +14/−1 ; diffcheck réussi, commit normal sans bypass, checkout lead propre. Pas de nouveau test ou fmt à revendiquer sur la seule sortie du hook.

Copie locale remise à l'auteur T3 : `.tmp/t3-inputs/contract-sentinel-ac6-approved.md`, SHA256 `1d2582037ab6a90d552e38f4eb3744fe7899ec0f056d4c776de09fc3276c116d`, source/copie comparées égales. Le code T3 non committé reste dans son worktree, aucun checkout/reset/merge imposé sur son arbre. Sa base ea2c009 plus cette version contractuelle intégrée est l'entrée explicite de la correction.

## Preuves indépendantes reçues pendant l'interruption

Summary du run `t3-http-043d197deb334621` lu : HTTP16PASS/0ignore/exit0 et résilience2PASS/0ignore/exit0, aucune cible non exécutée dans ce batch, issues[]. Ces comptes incluent chacun un test SMTP. owned_running[], avant3/après5, node préexistant running et quatre SMTP arrêtés. Ces résultats sont archivés du25octobre, pas un inventaire live après restart ; aucun Docker lancé par le lead. Rollup35créés/31retirés/4retenusask/10volumesask annoncé par auteur, pas recompté globalement ici. Pas de rerun HTTP/résilience/runtime pour reprise documentaire.

## Suite

Réactiver la même session t3-runtime pour l'exception autorisée, tests ciblés positifs/négatifs puis correction, AC6 réel inchangé. Le slot Cargo est libre selon absence de processus cargo lors du contrôle hôte ; il retourne exclusivement à T3/24jobs. Nouveau préflight/bail contemporain avant nouvelle IT (date26octobre), pas réutilisation d'un inventaire ancien comme état courant. Pas de publication globale ou promotion ADR ; revue indépendante après remise intégrée du correctif. Une indisponibilité réelle d'outils après restart doit être signalée, jamais contournée silencieusement.
