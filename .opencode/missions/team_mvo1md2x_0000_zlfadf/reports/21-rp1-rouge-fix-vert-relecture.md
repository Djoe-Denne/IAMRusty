# Rapport 21 — RP-1 reproduit, correctif local et relecture

## Preuves recoupées

Lead : HEAD129dcc1575e7dd59b4c7935de0b3794dbb7e7d07, source propre,
diff --check exit0 ; delta depuis eb685fe limité aux deux fichiers SQS, +207/-14.

Lecture des logs : témoin RP-1 avant fix = 4tests réels,2échecs sur H::drop
(enfant jamais pollé et parent actif annulé),2succès stop/panic. Ce sont des
assertions métier d'ordre de destruction, pas une panne de compilation/runtime.
Après fix : 16tests exécutés/passés,0ignorés,5horsfiltre. Logs conservés sous
.tmp/ac7-rp1-red.log et .tmp/ac7-rp1-green-1.log du worktree T2.

## Décision et suite

Ne pas lever le finding sur le seul témoignage de l'auteur ou les tests verts.
Relecture bornée du delta demandée au même pool-3/rust-perf-reviewer, même
session déjà rattachée à la source. Brief persistant avant relecture :
.cursor/review-briefings/20261025-ac7-rp1-recheck-129dcc1.md.
Pas de revue générale supplémentaire, pas de Cargo reviewer.

SHA local non publié, submodule toujours6c63b65, Telegraph intact selon auteur.
Publication/pin/atomicité restent bloqués jusqu'au verdict. Clippy antérieur
reste nonvert sur lib.rs hors diff, non relancé sans justification nouvelle.
