# Rapport 20 — revue AC7 : RP-1 bloque le pin

## Verdict et portée

Revue indépendante terminée sur eb685fe propre : BLOCK RP-1 P2,
HIGH_CONFIDENCE, ownership/concurrence. Pas de test effectué par le reviewer.
L'hypothèse concrète doit être reproduite ou levée par une garantie d'ordre
et un témoin discriminant ; elle n'est pas annoncée comme reproduite.

Le lease local _owner peut libérer active avant les captures handler/client
lors de la destruction d'un enfant après drop du parent. Un restart pourrait
être admis avant fin de destruction des ressources de l'ancien run. Les autres
mécanismes inspectés n'ont pas de nouveau finding établi.

## Actions et barrière

Briefing indépendant complet persisté avant correction :
.cursor/review-briefings/20261025T1755Z-rust-perf-ac7-sqs-lifecycle-eb685fe.md,
index actualisé. Le même propriétaire T2 doit ajouter un témoin H::drop observant
active, avant fix, puis corriger l'owner englobant à ordre de destruction garanti
ou réfuter précisément le finding. Cas non pollé, actif, stop/panic et réadmission
à couvrir ; pas de destructeur bloquant ni affaiblissement d'oracle.

Réutiliser les preuves valides, exécuter seulement les tests ciblés nécessaires,
slotCargo unique/jobs24 ; zéro Docker requis. Même reviewer pour relecture ciblée
du correctif, sans nouvelle revue générale ou nouveau membre. Ni pin ni mutation
Telegraph avant levée du blocage. Aucun retrait/force push de la branche distante.
