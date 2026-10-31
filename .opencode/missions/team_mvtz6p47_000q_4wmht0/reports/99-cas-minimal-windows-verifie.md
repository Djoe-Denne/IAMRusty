# Cas minimal Windows vérifié

## Résultat

**PASS fonctionnel** du cas k6/Chromium Windows : `about:blank`, calcul42 et version
Chrome 156.0.8078.73 égale au pin via l'API du même cas. k6 annonce 2.3.0. Le lanceur
finit avec le code 0 ; outillage, navigateur, nettoyage et confidentialité sont PASS.

Le résumé courant est validé par l'oracle strict : un cas attendu/démarré/achevé,
une itération, zéro requête k6/http et les trois checks browser-evaluation,
browser-blank, browser-version, chacun avec une réussite et aucun échec. Le répertoire
neuf est retiré ; aucun processus k6 ou Chrome n'est présent avant ou après le run.
Ce n'est ni un benchmark de charge,
ni une qualification du sandbox ou de Linux.

## Vérifications réellement réalisées

- `git diff --check` : code 0.
- `python.exe -B -m unittest discover -s ops/tests/k6/tests -p 'test_*windows*.py'` :
  **34 tests, 42.651 s, code 0, zéro skip**, exécutés par le lead sur le code final intégré.
  Sources inchangées pendant le run ; 11 identités de fixtures observées terminées,
  deux journaux frais recoupés et deux répertoires de fixtures absents.
- Mode navigateur Windows réel : code 0, 2874 ms pour la séquence outils, cas et nettoyage.
  Cette durée globale n'est pas une mesure de latence applicative.
- La version k6 et les deux `k6 inspect` ont réellement été exécutés dans le mode
  navigateur, sur les copies exactes des scénarios. Options : un utilisateur virtuel,
  une itération,
  per-vu-iterations, délais et seuils fonctionnels stricts validés. Pas de répétition
  séparée de ces mêmes commandes pour remplir le rapport.
- Mode tools autonome non exécuté séparément : ses prérequis sont déjà exercés par browser et
  sa limite CHROMIUM_VERSION_PENDING sans cas navigateur reste documentée.
- Chrome `--version` NOT_EXECUTED/INCONCLUSIVE : preuve effective par le check API
  ratifié, pas un autre lancement. Validate/run/selftest HTTP réels NOT_EXECUTED /
  INCONCLUSIVE, faute de cible Windows existante admise ; aucun faux endpoint créé.

## Correctifs conservés et limites

Les seules corrections après les vrais échecs sont le préfixe `k6.exe`, le marqueur
standard `group` dans la commande Windows pour reporter chaque assertion, le rejet
explicite des champs vides et les diagnostics privés bornés. Les tests rouge/vert
des deux défauts sont conservés. Ni JS partagé, HTTP, summary.js, pins ou workflow
n'est modifié par ces corrections. Les protections natives Chromium restent activées,
sans attestation supplémentaire ni affaiblissement TLS/sandbox.

HTTP et ses selftests restent INCONCLUSIVE ; WSL suspendu et Linux INCONCLUSIVE.
Les deux jobs Linux restent SUSPENDED/NOT_EXECUTED, même si GitHub affiche un Success
pour des jobs ignorés. Aucun Docker, Kind, WSL, déploiement, tunnel ou dispatch créé.
Les diagnostics des échecs restent privés hors Git ; aucun profil n'est retenu.

Publication et disposition des anciennes branches/worktrees restent à terminer ;
ce rapport n'annonce pas encore la clôture administrative de la mission.

## Références techniques

Base de preuve `12681ae` + delta reporting intégré, trois SHA256 source égaux à la
livraison `result_mvw7j9b6_0000_7rxc40`. Test ciblé reporting ROUGE1/exit1/0.408s,
VERT 1 / code 0 / zéro skip / 0.418 s. Les 34 tests finaux sont un run distinct, pas une somme
des anciens passages de 32 tests ou des historiques WSL.
Preuves privées : `.tmp/k6-browser-r3-20261031-retry3-lease.json` et
`.tmp/k6-windows-final-unit-20261031-lease.json`. Elles ne sont pas publiées.
Localisation : run-windows.ps1, helpers Windows et leurs tests ; impact limité au
raccord Windows. Aucun Rust/service/SDK modifié ; MCP symbolique Rust non applicable.
