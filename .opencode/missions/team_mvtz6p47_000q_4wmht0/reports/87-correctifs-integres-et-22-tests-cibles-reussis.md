# Correctifs intégrés et 22 tests ciblés réussis

Le but reste de vérifier les outils Windows sans exposer de données ni affaiblir
les protections. La livraison complète est récupérée : les dix fichiers intégrés
correspondent exactement aux empreintes de l'auteur. Le lead a exécuté **22 tests
ciblés, tous réussis et aucun ignoré**, sur ce contenu.

Ils couvrent la création exclusive du dossier et sa suppression liée à l'identité,
le contrôle des fichiers Chrome extraits contre l'archive authentifiée, ainsi que
la vérification de version dans le scénario réel sous entrées simulées connues.
Un jeton principal observé positif ne peut plus produire Chromium PASS.

Ces tests ne lancent ni k6 ni Chrome. Ils ont créé deux petits processus connus,
Node et Python, observés terminés ; leurs dossiers de fixture sont retirés et les
journaux privés conservés hors Git. Les preuves initiales inchangées restent à
leur portée, sans les additionner comme une nouvelle qualification globale.

## Limite toujours ouverte

La conservation des protections du code si un processus survit à l'arrêt gracieux
reste à trancher. La correction ne prétend pas résoudre ce point par un simple
verdict d'échec. **Aucun lancement réel ni publication n'est admis par ces tests.**

Une relecture indépendante ciblée des correctifs peut maintenant avancer pendant
que l'architecte examine cette durée de protection. Les verdicts Chromium, HTTP,
WSL et Linux restent distincts et non qualifiés comme précédemment.

## Preuves techniques

- Candidate complète result_mvve350f_0006_aruwie, delta83262octets/SHA46cd6051… ;
  flux compressé validé et données UTF8strict, patchs appliqués sans exécuter le delta.
- Dix fichiers : tailles et SHA256 tous identiques au manifeste final de l'auteur.
- Commande native `C:/Python313/python.exe -B -m unittest` depuis
  `ops/tests/k6/tests`, classes `test_security_windows.WindowsSecurityTests`,
  `test_browser_version_windows.BrowserVersionWindowsTests` et sept méthodes
  affectées de WindowsTests : **22/23.986s/exit0/0skip**.
- Méthodes reprises : résumés positif/nonvacant, pinsPE, refusreparse,
  entrée outils/navigateur sans pin, confidentialité du vrai callsite et
  chemins/reuse/rename. Child inchangé : ancienne preuve21tests réutilisée pour
  les chemins non affectés, pas suite historique relancée.
- Deux journaux privés frais, trois lignes/deux identités distinctes/deuxfinies,
  dossiers de fixture absents et anciennes entrées conservées.
- `git diff --check` global exit0 avant cette campagne. Pas de faux PASS de l'essai
  ancien interrompu de l'auteur, dont le nettoyage exhaustif reste UNPROVEN.
