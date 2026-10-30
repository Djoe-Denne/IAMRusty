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

## Complément suivant : familles fermées, après validation du delta783

Le complément précédent est intégré783fa80 et vérifié dans6b58108/run40982576736 :129Linux OK sans skip22/24, direct22 SIGABRT avant cleanup. Base du nouveau delta **783fa805d57339d7a8c5fc3a44022a2cdd639cf5**, même arbre/session ; le lead fournit le commit des présentes entrées avant reprise. Ne pas recapturer les deltas d47/0601/783 déjà intégrés.

Lire rapport34 et suivi expert dans `20261030-k6-signal-expert-result.md`. L'activation de ce petit complément est une décision lead dans le diagnostic Accepted ; une ancre source/assertion plus précise est instruite séparément par architecte, pas autorisée par cette tâche. Les deux branches sont indépendantes ; aucun code d'ancre ni nouvelle collecte ici.

Scope `ops/tests/k6/**` seulement, pas workflow/contrat/ADR/pin/argument/scénario/provisionneur/garde/délai/arrêt/proc ou reçu privé. Réutiliser les flux privés déjà capturés et bornés ; après contrôle complet de confidentialité, projeter uniquement une petite facette direct-only/versionnée, validée : présence stdout et stderr EMPTY/PRESENT, famille CHROMIUM_FATAL_REPORTED / ASSERTION_FAILURE_REPORTED / OTHER_TEXT / NO_TEXT / MULTIPLE. Formats des reconnaisseurs recoupés avec Chromium156 logging.cc et glibc2.35 assert/assert.c, sources du suivi expert, pas regex de mots vagues ni nouvelle longue allowlist.

Ne jamais projeter le texte, source/site, condition d'assertion, chemin, ligne, PID ou suffixe. ERROR seul, bruit, mots isolés, fragments répartis entre flux ou formats incomplets restent OTHER_TEXT, pas une cause. Privacy, UTF-8 invalide ou overflow retiennent la facette ; les refus/cleanup prioritaires et exits restent inchangés. FATAL/assertion sont « rapportés », pas causalité prouvée. Aucun export ni fichier publié ajouté.

Tests ROUGE puis GREEN Windows sur les fixtures existantes : deux flux vides, présence séparée, formats exacts FATAL/assertion, cas multiples, bruit non fatal/fragments/mots isolés, sentinelles/UTF8/overflow, facette forgée, native0 et failures toujours non qualifiants/non publiés. Conserver tous129tests et contrôle de non-modification Child/gate/CI ; pas nouveau harness, survivant réel ou runtime.

Actualiser aussi l'entrée « Current admission » du README avec la photographie vérifiée :129tests Linux22/24 et outils exécutés, HTTP/live selftests/browser toujours non qualifiés. Les anciennes phrases « candidate/Linux execution unproven » et l'ancienne base d'auteur ne décrivent plus l'état courant. Garder l'historique utile dans IMPLEMENTATION/ledger, ne pas requalifier ces résultats en mission PASS.

Condition d'arrêt : ce lot livre seulement le code et preuves locales, source gelée. Aucun essai par l'auteur. Le lead n'exécutera pas une série de reconnaisseurs après OTHER_TEXT/NO_TEXT/FATAL insuffisant ; l'arbitrage d'ancre séparé doit alors gouverner la suite. Remonter avant code si ce scope exige une nouvelle frontière. Pas Cargo/hook/runtime/CI/push/ledger partagé.
