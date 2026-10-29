# Revue delta bornée — répertoire de sortie privé sous Linux

## Mandat READONLY

Même revue indépendante, limitée au delta db16aa726ca0f44876180ac45b4922110c030c09..db352be1ebe01eb275fd74d8cdee6f7298b99bf1. Trois fichiers : ops/tests/k6/lib/safety.py, tests/test_boundaries.py et IMPLEMENTATION.md. Ne pas refaire l'audit initial ni lancer runtime/Cargo/install. Examiner si les invariants de propriétaire/droits/canonicalité/non-liens/hors-Git/sortie neuve restent respectés avec la validation du parent immédiat. Rapporter uniquement défauts démontrés, suggestions séparées et limites.

## Contrat et faits

Le contrat accepté exige une sortie neuve sous parent privé canonique appartenant à l'utilisateur. L'autorité historique, la cible HTTP, les pins et le workflow ne changent pas. Pas de nouvelle URL, booléen d'autorisation ou mode TLS/sandbox.

Run lead réel avant correction sous Ubuntu24.04/WSL2 x86_64, artiste/UID1000 :37tests, exit1,1échec/2erreurs, PATH_INVALID avant les processus attendus ; preuve au briefing 20261029-k6-linux-first-run-path-invalid.md. Tests Windows initiaux avaient5skips POSIX.

Après récupération ciblée par patch et comparaison octet pour octet des trois fichiers au candidat, le lead a rejoué sous le même Linux non-root :40tests, exit0, aucun skip,0échec/erreur. Log contemporain .tmp/k6-linux-unittest-1793309664423.log. Cette preuve porte sur les tests du dispositif, pas k6/Chrome/HTTP. Le commit db352be a passé diff-check et vrai hook cargo fmt, aucun WIP étranger inclus.

Nouveaux tests de politique avec metadata injectée sur Windows et régression POSIX réelle ; négatifs sentinelle/overflow attendent maintenant leurs codes exacts. Vous pouvez vérifier leur caractère discriminant à partir du diff ; aucun test de parsing n'est présenté comme preuve live HTTP.

Les contrôles de fichiers et ceux des répertoires ont des responsabilités différentes à examiner. Ne pas assouplir les gardes de cluster ou les contrôles de liens/propriétaire au motif de cette correction. L'activation HTTP reste refusée selon l'addendum accepté.

## Sortie

Verdict ciblé sur ce delta, fichier/ligne et scénario si défaut ; aucun correctif demandé automatiquement en l'absence de finding. Les conclusions du développeur ne sont pas une preuve et ne sont pas nécessaires pour la revue. Préserver l'indépendance de cette relecture.
