# 11. Deux corrections ciblées avant le premier run

## Objectif et étape

La campagne doit exécuter fidèlement les scénarios et restaurer le bon certificat même si une phase échoue. L'inspection native du raccord et la revue indépendante ont trouvé deux écarts concrets. Ils sont corrigés dans le lot parent existant, sans nouvelle architecture ni campagne du framework.

## Inspection réelle : phase SAN non reçue

Le parent Windows a exécuté la versionk6 puis inspecté le scénario navigateur avec succès. L'inspection suivante échoue avant toute requête parce que la phase SAN est absente au moment de l'initialisation du script. k6 renvoie107. Sa trace d'erreur contient un chemin source privé : le garde de confidentialité refuse le résultat et le parent sort2, sans publier ce flux brut.

Trois processus k6 observés terminés, capture complète, nettoyagePASS ; inventaires k6/Chrome0→0. Aucun Chrome, HTTPcible, certificat ou laboratoire appelé. Les diagnostics privés restent volontairement conservés, pas un nettoyage incomplet de processus. Ce n'est ni un PASS d'inspection globale ni un défautproduitIAM.

Correction demandée : transmettre explicitement la seule phase fermée à inspect, pas affaiblir le script avec une valeur par défaut ni désactiver le garde. Une inspection gardée sera rejouée après correction, pas une suite de tests du framework.

## Revue sécurité : réparation empêchée par l'état de santé

La revue ne trouve aucun défautCRITICAL/HIGH mais un problème local de restauration : si le PATCH du mauvais certificat réussit puis le rollout laisse IAMnonReady, la réparation exige elle-même un PodReady avant de tenter son PATCH. Le mauvais certificat peut rester installé. Le parent ne produit pas de fauxPASS, mais le signalement seul ne remplit pas le critère de restauration.

Correction bornée : le PATCH réparateur doit garder la cible/propriété/UID/tests de bytes et la jonction de l'opération précédente, sans dépendre de la readiness d'IAM/fournisseur/Envoy. Les acquittements et la reprise sont vérifiés après réparation. La revue est statique ; aucun échecpartielréel n'a été provoqué ni faussepreuveROUGE revendiquée.

## Décision et suite

Ces deux corrections sont confiées au même auteur. Le défaut de restauration bloque le stimulusSAN et le démarrage du laboratoire dans la séquence actuelle ; les travaux de correction, d'inspection et de préparation non mutante continuent. Aucun GOutilisateur supplémentaire n'est requis pour ce correctif conforme au mandat. Pas nouvellefixture, framework, suite0412 ni campagne de robustesse.

## Annexe de preuves

Codeexaminé51e74e1. Revue sécurité persistée .cursor/review-briefings/20261031T1637Z-security-k6-auth-parent-51e74e1.md, findingS-1MEDIUM/confianceHIGH, candidatresult_mvwmkvv6_0011_q4e2wy.

Inspection réelle via script temporaire .tmp/20261031-lead51e74e1-auth-inspect.ps1 et parent -Modeauth-inspect, commandes et chemins complets enregistrés avant appel dans .tmp/k6-auth-inspect-dca0cec54e7645838d3ec021019b39f0-plan.json ; résultat publicfermé dans fichier-result.json correspondant. Processus3 : exits0/0/107, Active0/Cleanuptrue/CaptureCompletetrue/Overflowfalse. Stderr269octets analysé sans publication : AUTH_SAN_PHASE_INVALID et cheminprivésource présents.

Deux essais du wrapper depuis l'environnement Context Mode ont précédé le run natif : premier cheminGitnonrésolu, puis codeLASTEXITCODE noncollectable, pas résultatparentaccepté ; plan789612957d874f2c99da41283d0113ed conservé et statutnonqualifié. Exécution native directe PowerShell ensuite probante avec le résumé fermé ci-dessus. MCPContextMode fonctionne pour l'analyse privée ; aucune panneglobale ou permission contournée. Aucun de ces essaisinvalides ne compte comme scénarioexécuté.
