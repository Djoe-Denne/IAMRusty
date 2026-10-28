# Rapport 28 — T3 : premiers contrôles, IT non lancées

## Avancement rapporté par l'auteur

t3-runtime confirme localement HEAD ea2c009 et pin129dcc1 dans le bon worktree. Workflow MCP local chargé, références standalone/monolithe examinées via Serena canonique avec sources réelles du worktree comme réalité.

Écrits localement : dispatch uniquement Live et non-NoOp, suspension sans claim, supervision de tous les handles et deadline globale6s incluant les stops.

Commandes/résultats annoncés, non encore recoupés par le lead : check setup et monolithe réussis ; six tests unitaires setup exécutés, tous PASS, zéro ignoré ; quatre cibles IT compilées avec --no-run. Ces compilations ne prouvent pas les scénarios IT.

## Échec et traitement borné

Deux premiers tests monolithe échouent sur une fixture IAM vide invalide : le helper IAM exige un handle. Correction annoncée uniquement dans le test, ajout d'un handle IAM pending possédé, oracle conservé ; relance ciblée à recevoir. Ce n'est pas une preuve rouge de production. Le futur pending ajouté au test doit rester possédé et être détruit/joint à la fin, y compris en échec ; la borne globale et l'oracle de destruction ne doivent pas être affaiblis.

## Suite et limites

Preuves locales sous `.tmp/t3-evidence/` du worktree auteur ; exiger commandes exactes et logs lors de la remise pour recoupement. Un seul Cargo Windows24jobs, aucun Cargo concurrent du lead. Aucune IT ni ressource runtime créée annoncée à ce stade. Préflight fixtures/lifecycle/evidence et bail à recevoir avant IT. Pas de PASSglobal, preuve verticale et arrêt réel encore incomplets ; review indépendante et documentation attendent toujours T3 intégrée.
