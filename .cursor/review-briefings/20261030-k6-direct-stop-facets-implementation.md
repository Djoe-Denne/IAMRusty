# Deux facettes d'observation directe — petit complément sans changement de contrôle

## Base et mandat

Même auteur/session pool13, delta contre `0601f6f73a93a2fedb9b8717450d06acd6a5fa67` ; son arbre physique b316 contient déjà les livrables d47/0601 capturés. Vérifier cette équivalence avant édition, pas reset/rebase/index ni recapture aveugle d'un diff b316. Lire rapport31 et consultation `.cursor/review-briefings/20261030-k6-signal-expert-result.md` canoniques, intégrés avant reprise par le lead.

La chaîne de code démontre que le signal primaire direct22 de40979730197 précède le cleanup, sans donner le signal exact/émetteur. Recommandation experte compatible Accepted : deux facettes fermées seulement, pas nouveau harness, arguments, pin, politique ou classification d'erreur spéculative.

## Ownership et invariants

`ops/tests/k6/**` seulement. Pas workflow, contrat/ADR, lock, scénario, provisionneur ou service. Ne modifier aucune condition de sandbox, lecture /proc, signal envoyé, délai, priorité d'erreur, contrôle privacy/overflow/cleanup ou admission du reçu privé22.

- Mémoriser dans Child le chemin de fin au moment réel : statut primaire observé avant cleanup, refus précoce de garde, timeout, interruption, dépassement, autre erreur/non observé. Labels fermés ; ne pas inférer la phase après coup depuis le seul code final et ne pas la perdre si cleanup/privacy devient prioritaire.
- Projeter le signal précis à partir de la valeur entière négative du primaire, par table Linux fermée de signaux connus ; absence, sortie sans signal, signal inconnu et type invalide distingués/refusés sans valeur brute. Ne jamais inventer émetteur ou cause.
- Projection **direct-only**, validée et bornée. Préférer une petite facette séparée/versionnée ou autre ajout compatible pour ne pas élargir le schéma exact du reçu k6, son prédicat, la CI ou les anciennes observations. Pas de texte/argv/path/PID nouveau, aucun fichier publié ni PASS fonctionnel ; native0 reste non qualifiant.
- Conserver les faits internes même si leur projection doit être retenue sur un échec de confidentialité/overflow. Toute observation forgée reste rejetée ; aucune métadonnée non fiable recopiée. Choisir le détail local le plus simple, pas de machine à états générale.

## Tests utiles

ROUGE puis GREEN Windows avec temp approuvé. Réutiliser fake_child_reads/fixtures existants en exécutant le vrai Child.execute, pas seulement un Mock positionnant les nouveaux champs : négatif primaire observé avant cleanup, refus de garde suivi du TERM simulé, timeout/interruption avec statut final négatif, overflow/privacy/cleanup ratés conservant le motif et leurs verdicts. Assertions sur phase avant/après, absence de qualification/publication et valeurs forgées/connues/inconnues, sans vrai survivant ni kill forcé.

Les118tests précédents sont historiques pour ce delta ; le lead les rejouera avec les nouvelles régressions après intégration. Pas de Linux/Chrome/CI/runtime/Cargo/hook/installation/push ou ledger partagé par l'auteur. Rendre rapport complet, fichiers/delta exact, preuves ROUGE/GREEN et limites, puis geler le candidat. Si un contrôle doit changer, arrêter ce correctif et remonter la frontière au lead/architecte avant code.
