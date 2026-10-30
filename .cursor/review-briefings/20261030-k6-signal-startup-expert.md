# Consultation technique READONLY — attribuer le signal sans relâcher les gardes

## Objectif

Mission1 : socle k6 Linux, un GET public IAM et qualification Chromium distincte, pas charge/OAuth/POST/service/identité/déployeur/tunnel. Aucun PASS global acquis. Problème devenu difficile après plusieurs expériences discriminantes : les captures fermées localisent les refus mais perdent l'origine exacte du signal22. Chercher une observation minimale plutôt qu'une nouvelle architecture ou un essai par tâtonnement.

Lire le rapport30 et le ledger canoniques, ADR0504/V1 Accepted/Partial et contrat k6-functional-linux.md : comparaison22/direct séparé Accepted, AppArmor durable Proposed/non autorisé. Source publiée0601f6f dansed87f65 ; nouveau run40979730197 terminé. Aucun raw stderr disponible après le job : les flux restent privés, seuls labels/JSON bornés et non publiants sont accessibles. Ne pas demander leur export ou contourner la confidentialité.

## Faits utiles

-118tests Linux OK sans skip sur22/24, outils/versions et deux inspect passent. Binairek62.3.0 et CfT156.0.8078.73, archives/ELF hash vérifiés avant chaque exécution. k6 s'arrête avant corps JS : expected1/started0/completed0/iterations1/requests0, summaryVALID, renderer absent, namespaces NOT_OBSERVED ; k6_exitSCRIPT_ABORTED selon table officielle, pas99.
-24 : restriction AppArmor1, clone1/limite positive, parent seccomp0/NoNewPrivs0 ; direct NoUsableSandbox/SIGNAL/EXITED/privacyFAIL, enfants cleanupPASS.
-22 : restriction0, mêmes autres labels, parent uid1001/kernel6.8.0-1070-azure ; k6 même refus. Direct exécuté après reçu privé courant, même job/host/pins, catégorie nativeUNCLASSIFIED/SIGNAL/EXITED/privacyPASS, enfants cleanupPASS ; durée totale courte, pas CHILD_TIMEOUT/INTERRUPTED rapporté. Le signal précis et son émetteur ne sont pas projetés.
- Source : run-linux.py:direct_chromium/scenario_run/admissible_browser_refusal/main, lib/safety.py:Child.execute/inventory/sandbox_witness/browser_diagnostic. Child conserve l'exit primaire, peut envoyer SIGTERM aux identités possédées dans son finally ; SANDBOX_UNPROVEN peut être levé par flag interdit observé ou par absence finale de témoin. Ne pas présumer que tout SIGNAL signifie crash natif, ni qu'un témoin absent est une sandbox désactivée.
- Parser k6 peut perdre FATAL ; source2.3 BrowserOptions ignore ARGS vide, defaults vérifiés sans no-sandbox. Hypothèses env-executable filtré et ARGS vide désactivant les protections déjà écartées par source. Direct24 et direct22 n'ont pas la même catégorie, donc pas une cause commune prouvée.

## Demande bornée

1. Examiner les chemins d'arrêt/erreur actuels et sources officielles épinglées pertinentes ; repérer une erreur de diagnostic/supervision démontrable s'il y en a une, sans conclure depuis un seul label. Réutiliser les preuves et recherches valides ; chercher sur Internet avant toute recommandation de nouvel essai.
2. Proposer au plus quelques hypothèses hiérarchisées avec faits contre/pour et une expérience réellement discriminante. Comparer notamment une projection fermée du signal précis/de la raison d'arrêt parent avec une classification minimale des erreurs natives ; éviter une longue allowlist spéculative ou une collection de probes.
3. Recommander le plus petit delta justifié et ses oracles, risques et limites. Pas de code, nouveau harness, stderr brut, package, strace/unshare, changement d'arguments/pin/politique ou privilège exécuté. Si frontière structurelle ouverte, la décrire pour architecte avant code, pas la trancher silencieusement.

Les pistes Crashpad/config/cache non inscriptibles et libXtst ont été recherchées après le dernier essai, pas établies. Le dernier ticket concerne Chrome151/images custom, fermé ; notre --version passe. Ne pas recycler no-sandbox, sysctl global, helper SUID, image flottante, conteneur privilégié ou downgrade durable22 comme solution.

Rendre conclusion concise, sources vérifiées, Localisation/Impact/Limites et recommandation bornée au lead, sans écrire de production/contrat/ledger, runtime/Cargo/CI/téléchargement d'exécutable ou sous-agent. Le lead reste coordinateur ; user autorise les essais supplémentaires raisonnés/recherchés, pas une relaxation des contraintes.
