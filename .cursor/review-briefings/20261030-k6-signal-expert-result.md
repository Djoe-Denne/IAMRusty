# Consultation technique — signal direct22 et chemin de fin

Consultation READONLY, pas verdict QA de qualification. Candidat complet `result_mvu8qe4t_0019_2k471v`, session `ses_e75c785f2ffeAdzaqXUyntuFq9`, tâche `task_mvu8m6ff_0014_5w8ct0`, HEAD documentaire367a059/source0601 inchangée.

## Conclusion vérifiée

Pour le tuple du run40979730197 — direct22 SIGNAL, SANDBOX_UNPROVEN, exit2, confidentialité/cleanup PASS — le SIGTERM du propre cleanup Child n'explique pas le signal primaire. Le signal exact et un éventuel émetteur externe restent inconnus ; aucun bug d'arrêt démontré.

Preuve de contrôle :

1. safety.py:289–294 lève Rejected(SANDBOX_UNPROVEN) lors d'un flag interdit ; inconclusive=False par défaut (29–34). L'exception devient failure (437–438), est relancée après cleanup avant le refus final (514–517), conservée par le direct (run-linux.py:375–377), puis sort1 (847).
2. La boucle normale exige primaire terminé et flux fermés (434–436), mémorise son returncode avant finally, puis seulement effectue les TERM de cleanup (449–452/463–466). La récupération tardive (493–496) ne remplace pas le code déjà mémorisé. Après privacy/cleanup et absence de failure, le manque de témoin produit Rejected(SANDBOX_UNPROVEN, True) (518–522), donc exit2.
3. Le workflow lance directement Python et ne normalise pas son exit. termination=EXITED dit seulement qu'un statut final est disponible, pas une sortie normale ou un crash logiciel prouvé.

Le lead a recoupé ces branches, le défaut inconclusive=False et le prédicat avec Serena sur la racine canonique, puis les extraits de boucle/cleanup et tests connus. Les références LSP à sandbox_witness renvoient Child.execute ; grep exact complète avec test_boundaries.py:408–413, non retourné par cette référence dynamique. Aucun index n'est pris comme preuve d'absence de test.

## Hypothèses et prochain delta recommandé

Arrêt fatal volontaire de Chromium ou défaut natif précoce compatibles ; signal externe non exclu. L'émetteur n'est pas dans le statut POSIX. Refus24 NoUsableSandbox distinct ; restriction0 sur22 ne prouve pas la création effective des namespaces. SCRIPT_ABORTED108 appartient à k6 et peut résulter d'un Abort interne d'initialisation, pas d'un appel JS test.abort ou d'une cause Chromium prouvée.

Ajouter seulement deux facettes fermées : signal précis et chemin de fin Child. Capturer le motif au moment de l'événement, sans le remplacer si privacy/cleanup devient prioritaire. Ne changer ni garde, lecture /proc, arrêt, argument, pin ou politique. Ne pas inventer de champ émetteur ou de longue allowlist d'erreurs natives spéculatives.

Oracles : sortie négative observée avant cleanup distincte du refus précoce de flag ; timeout/interruption gardent leur motif malgré le signal final ; privacy/overflow/cleanup ne deviennent pas verts ; métadonnées forgées ou signal inconnu restent fermés/non publiés. Fixtures existantes, pas nouveau harness.

## Sources officielles consultées

- https://docs.python.org/3.12/library/subprocess.html#subprocess.Popen.returncode : -N signifie terminaison par signalN ; poll fixe le statut, pas son émetteur.
- https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser_process.go : CommandContext/annulation/erreurs distinctes, pas équivalence exit k6→cause Chrome.
- https://github.com/chromium/chromium/blob/156.0.8078.73/content/browser/zygote_host/zygote_host_impl_linux.cc : chemins fatals de démarrage multiples.
- https://github.com/chromium/chromium/blob/156.0.8078.73/base/immediate_crash.h : int3/ud2 et SIGTRAP/SIGILL ; un éventuel SIGTRAP ne prouverait pas un débogueur externe.

Expert : rev-parse HEAD367a059, diff-check exit0, diff/status outillage depuised87 vides. Aucun unittest, binaire, testLinux, CI/runtime/téléchargement ou écriture. Preuves118Linux du rapport30 réutilisées, aucune qualification ajoutée. Compatible avec addendum Accepted tant que le delta reste purement observabilité ; changer prédicat, arrêt, lectures système ou privilèges requerrait architecte.

## Suivi après SIGABRT observé — recommandation et condition d'arrêt

Candidat `result_mvu9thwl_0000_1wcyyv`, même session, source783 publiée6b58108, HEAD documentaire53e40ad. Rapport33/brief/ledger relus. Run40982576736 : direct22 corrélé, NATIVE_OBSERVED_BEFORE_CLEANUP/SIGABRT, privacy et enfants cleanup PASS, cause lexicale UNCLASSIFIED. Le propre cleanup Child reste exclu comme cause du signal primaire ; pas de rétroattribution aux runs antérieurs ni de preuve d'émetteur.

Lacune vérifiée : browser_diagnostic combine les flux, recherche quelques messages puis rend UNCLASSIFIED. Ce label couvre deux flux vides, un seul flux présent, du texte non reconnu ou un FATAL non couvert. On ne sait pas encore si un diagnostic exploitable existait. Pas de défaut de Chromium ou du superviseur démontré par ce seul constat.

Recommandation : pas d'essai limité à vide/non-vide. Petite classification fermée des formats déjà capturés, après confidentialité complète : stdout/stderr EMPTY ou PRESENT ; famille CHROMIUM_FATAL_REPORTED, ASSERTION_FAILURE_REPORTED, OTHER_TEXT, NO_TEXT et éventuellement MULTIPLE. Ni chemins, assertion, numéros de ligne, PID ou suffixes projetés. Un ERROR Chromium n'est pas une cause. Mots isolés, fragments et bruit restent OTHER_TEXT ; familles multiples sans priorité causale inventée ; privacy/UTF-8 invalide/overflow retiennent la nouvelle projection. Aucune modification d'exit, cleanup, reçu/gate ou verdict.

Condition d'arrêt : OTHER_TEXT ou absence de texte n'autorise pas une série de nouveaux reconnaisseurs spéculatifs. Même un FATAL seul peut nécessiter une ancre causale précise ; projection du site/assertion à faire arbitrer par architecte avant code, sans hash arbitraire ou ligne native exportée implicitement.

Hypothèses : abort volontaire du programme/bibliothèque compatible, refus fatal Chromium encore possible, signal externe non exclu. Aucune sonde système nouvelle justifiée pour l'émetteur. Piste chmod sans défaut statique identifié ; métadonnées ZIP ne prouvent pas les fichiers runtime.

Sources officielles versionnées vérifiées par l'expert :

- https://github.com/chromium/chromium/blob/156.0.8078.73/base/debug/debugger_posix.cc : chemin DEBUG_BREAK utilisant abort.
- https://sourceware.org/git/?p=glibc.git;a=blob_plain;f=assert/assert.c;hb=glibc-2.35 : format d'assertion stderr suivi d'abort.
- https://github.com/bminor/glibc/blob/glibc-2.35/stdlib/abort.c : raise(SIGABRT).
- https://github.com/chromium/chromium/blob/156.0.8078.73/base/logging.cc : en-tête avec sévérité/localisation, HandleFatal et ImmediateCrash.

Ces mécanismes possibles ne prouvent pas la bibliothèque ou branche effectivement utilisée. Commandes expert : rev-parse HEAD53e40ad, diff-check exit0, diff/status outillage depuis6b58108 vides.129Linux et preuves CI réutilisées, aucun nouveau test, essai, ressource ou écriture.
