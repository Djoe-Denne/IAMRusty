# Rapport 50 — source SDK présente, manifeste unifié et entrées accessibles

## Source SDK vérifiée, aucune réparation

T4 signale ENOENT sur rustycog-http/Cargo.toml et demande source de l'interaction HTTP/axum. Lead vérifie via Context Mode : HEADlead aa400f3/status propre, gitlink129dcc1, gitfile SDK rattaché aux métadonnées worktree ti et core.worktree correct. builder.rs existe13734octets ; blob local1c3a8e2035056322ee4169bef9e0d0aa4d8580c9 identique au pin. Cargo.toml racine SDK existe, blob8e45e35202ead87d00813f6e23acf1fce72933ea identique au pin.

git ls-tree confirme **absence de rustycog-http/Cargo.toml dans le pin**, et non disparition d'une source : SDK crate unifiée rustycog-framework. Dépendance axum dans rustycog/Cargo.toml195 =0.8, Cargo.lockapplication résout0.8.9 pour cette branche, autre0.7.9 également présente. Aucun deinit/checkout/restauration/move du SDK effectué. Ne pas prétendre disparition ou panne globale sur un fichier qui n'est pas suivi.

Même reviewer réactivé avec ces faits, sources réelles accessibles, sans verdict prérempli. Sa persistance de briefing est confiée au lead conformément au contrat Ensemble : il retourne le texte complet, pas d'écriture concurrente ou nouvelle question humaine nécessaire.

## Alias d'entrées locales

Les anciens retours T5 précédaient les copies `.tmp/t5-inputs`. Pour les chemins littéraux attendus par les profils, alias-copies supplémentaires créées sans écraser d'existant : INDEX et ledger local côtéT4, `.tmp/telegraph-t5-inputs.md`, INDEX et ledger côtéT5. Hashes source/copie vérifiés. Le ledgerT4 est l'extrait factuel filtré pour l'indépendance, pas l'historique auteur ; T5 reçoit les faits nécessaires et aucun verdictT4. Les archives/politiques locales déjà fournies restent valides.

Pas de code applicatif modifié, test relancé, Cargo/runtime ou diagnostic infra. T4 et T5 restent en parallèle ; aucun résultat de revue ou documentation intégré à annoncer à cette étape.
