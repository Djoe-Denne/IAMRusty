# Refuser le FATAL concurrent incomplet — correction bornée

## Contrat et finding

Reprise du même auteur13 après persistance de la revue. Lire le rapport complet
`20261030T1243Z-security-k6-private-site-eb3d355.md`, le rapport50 et le contrat
causal Accepted local/collecte conditionnelle, ancre publique Proposed.

S-1 : `_private_site_candidate` compte seulement les correspondances FATAL
complètes. Un complet plus un concurrent incomplet/non conforme peut garder une
unicité apparente. Le lead exige correction avant collecte. Test de régression
associé T-1 ; constat statique du reviewer, pas de PoC réelle exécutée.

## Source, périmètre et invariants

Base intégrée `968592f2efec114d6f7c0c1c52c84fb07b505cab` ; les quatre fichiers du
candidat sont déjà récupérés byte-identiques et sauvegardés. Vérifier cette égalité
avant edit. Ne pas appliquer le diff accumulé du HEAD physique b316. Préserver
source/index anciens ; backup22fichiers+index dans le ledger.

Écriture autorisée uniquement run-linux.py et tests/test_private_site.py sous
ops/tests/k6, petite précision README seulement si le comportement l'exige.
Le module privé, Child, workflow, pins, scénarios, provisionneur et classificateur
public direct_text restent inchangés. Aucun nouveau reconnaisseur positif,
concaténation de flux, stockage/affichage de leurs octets ou permission supplémentaire.

Ajouter un veto conservateur propre au consommateur lorsqu'un candidat FATAL
concurrent est incomplet ou non conforme ; conserver le positif unique. Un refus
plus conservateur n'est pas une nouvelle famille de diagnostic public.

## Preuve attendue

Avec les fixtures existantes, T-1 doit échouer avant correction : complet puis
second incomplet, ordre inversé si pertinent, même flux et flux distincts.
Après correction : zéro relevé, exit non nul conservé, aucune publication ou
donnée native dans les sorties. Vérifier aussi le positif unique, deux complets,
UTF-8/secret/overflow, cleanup et interruption déjà couverts.

Rendre ROUGE/GREEN réel puis suite Windows complète et diff-check avec nombres,
skips et limites exacts. Les162tests Linux0skip de la source968 sont historiques
pour ce correctif ; le parent vérifiera le delta conservé sous Linux.

## Limites

Pas de WSL, Chromium, k6, CI, installation, collecte réelle, Cargo/hook, push,
autre service, contrat/ledger partagé ou subagent. Ne pas utiliser de site natif
inconnu pour reproduire le finding. Aucun parent_check fixture ne devient réel.

Livrer delta exact contre968, nouveaux oracles et choix du veto, puis geler.
Une relecture limitée de fermeture portera sur ce seul finding. Deux tentatives
substantielles sans progrès vérifiable imposent le transfert habituel, pas un
élargissement de reconnaissance ou de harness.
