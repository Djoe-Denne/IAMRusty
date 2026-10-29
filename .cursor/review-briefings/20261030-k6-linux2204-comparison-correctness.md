# Revue indépendante — comparaison Ubuntu22.04 non qualifiante

## Mandat fresh READONLY

Diff exact `b316668d79a113281477eb799fb5f20fba5506ab..d47b80508f71cfffbbd5d6caf2146f949997a954`. Cinq fichiers : run-linux.py, workflow k6 dédié, README, nouveau test_linux_comparison.py et une ligne de portabilité du mock dans test_direct_chromium.py. Aucun changement du lock/pins, scénarios, provisionneur, safety.py ou service.

Revue correctness bornée au nouveau contrôle de plateforme et aux résultats observables : la comparaison22 ne doit pas ouvrir les commandes nominales, atteindre HTTP, publier des fichiers ou donner HTTP/Chromium PASS. Vérifier aussi la réutilisation des inspections extraites, les métadonnées et les préconditions d'un cas complet, la confidentialité/cleanup d'échec, et le raccord CI sans masquage de l'ancien job.

## Canon et faits vérifiés

ADR0504 et V1 Accepted/Partial. Contrat `docs/contracts/k6-functional-linux.md` : comparaison22.04 seule Accepted avant code dans b316668 ; Ubuntu24.04 cible nominale, future exception AppArmor Proposed et NON autorisée. La comparaison vise une dépendance à l'environnement, pas AppArmor seul. Même un cas et un témoin réussis sur22 restent non qualifiants, non-zéro et non publiés. Un k6/browser22 au plus, pas de direct22 demandé. Réexécution incidente du job24 lors du push admise, sans skip/continue-on-error ou A/B mono-variable revendiqué.

Le lead a récupéré les cinq fichiers par patch puis vérifié leur identité byte à byte avec le candidat. Préfixe du workflow b316 strictement conservé. Replay Windows `C:/Python313/python.exe -B -m unittest discover -s ops/tests/k6/tests -p test_*.py` :109tests/exit0/7skips POSIX,102exercés ; temp hôte approuvé. Diff-check normal et staged exit0 ; hook normal cargo fmt réussi lors de d47b805, slot Cargo constaté libre avant le commit. Aucun nouveau résultat Linux ou navigateur réel.

## Invariants à vérifier

- Commutateur fermé browser seulement ; nominal22 refusé, comparaison hors22/root/architecture inconnue refusée avant création d'output/enfant. Pas de sélecteur libre ou repli silencieux.
- Pins/identités/versions et deux inspections avant le browser, un lancement au plus, même scénario/Child/environnement. Inspect ne lance pas d'itération ni de requête.
- Cas COMPLETE et sandbox OBSERVED exigent le succès des contrôles réels existants, pas seulement exit0. Résumé absent/périmé/tronqué, cas zéro, check faux, témoin absent, interruption, privacy ou cleanup échoués ne peuvent être qualifiants.
- HTTP/Chromium toujours INCONCLUSIVE et exit non nul en comparaison, y compris positif ; aucun appel publish, target HTTP ou direct Chromium. Métadonnées bornées et cohérentes avec les versions/pins réellement contrôlés, provenance24 nominale non dégradée.
- Nouveau job22 seulement : acquisition explicite, actions SHA/contents:read/aucun credential, racine privée et aucun upload/dispatch/privilège/paquet. Ancien job24 et son échec restent visibles.

## Retour et limites

Findings démontrés avec scénario, fichier/ligne et contrat observable ; suggestions et limites séparées. Ne pas traiter les simulations comme preuve Linux et ne pas réauditer toute la campagne précédente. Pas d'édition production/test/ledger, runtime, Cargo, téléchargement, CI ou délégation. Réutiliser les preuves valides ; un test supplémentaire n'est nécessaire que pour établir un finding concret et autorisé. Rendre les gaps de tests bloquants T-<n> distincts des suggestions.
