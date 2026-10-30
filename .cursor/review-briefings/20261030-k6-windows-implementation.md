# Raccord k6 Windows — contrat de travail de l'auteur

## Admission et résultat attendu

Mission1, révisionnative2 approuvée par l'utilisateur le2026-10-30, hash
`4d651a945ab4f6757f24117ed715c37cf82fc712bf5b22721d593240543cac64`.
CatalogueOpenAI, choix/réglage reasoning natif et QAproportionate applicables ;
aucun override de fournisseur/modèle ou score manuel. Tâche planned/mixed,
contextefresh, écrivain isolé. Pas de sous-agent ou coordination par le worker.

Canon principal absolu `C:/Users/djden/source/repos/AIForAll`. Lire la présente
version intégrée et `docs/contracts/k6-windows-functional.md` **Accepted** avant
code. Base fonctionnelleLinux/anciensparentsf279e713 ; HEADauteur fourni par
l'admissionnative après commit de ce brief. Les ADR0504/V1 restentLinux, ne pas
les superséder. Les sourcesWSL archivées sont inactives et ne sont pas des entrées.
Ledgerparent `.cursor/handoffs/20261029-k6-01-socle.md`, lecture seulement.

Objectif : petit adaptateurWindows natif, versions/inspect et casabout:blank,
résultats fermés et nettoyage prouvé. Pas framework, deuxième superviseur,
réécriture globale du harness ou instrumentationFATAL. Les tests ne prouvent pas
un run réelk6/Chrome et un succèsJavaScript ne prouve pas le sandboxWindows.

## Périmètre et ownership

Autorisés : nouveau `ops/tests/k6/run-windows.ps1`, helpersWindows locaux minimaux
si nécessaires, tests `ops/tests/k6/tests/test_*windows*.py`, documentationWindows
dansREADME/IMPLEMENTATION. Un seul auteur pour ces fichiers couplés. Réutiliser les
troisJS sans changement fonctionnel, les validations pures seulement si neutres
et testées. Aucun `run-linux.py`, ChildPOSIX, conducteur privé ou parserWSL modifié.
Ni Rust/services/identité/OAuth/authz, seed/POST/charge, sysctl/ACLglobal, Docker/
Kind/WSL, GitHubdispatch/VM, nouveau protocole cible ou outil de collecteprivée.

Le lead possède indépendamment `.github/workflows/k6-functional-smoke.yml`
(suspensiondes2jobsLinux) et la provenance/acquisitiondesassetsWindows. Ne pas
modifier ces fichiers ou démarrer ces opérations en double. Aucun push par auteur.
Pas Cargo(hook réservé au lead), format global, installation ou acquéreur automatique.

Pins finaux sous `config/tools.windows.lock.json` ou nom convenu : le lead établit
les octets officiels enparallèle et intègre le lock après livraison du raccord ;
ne pas inventer unhash ni utiliser un placeholder comme pin valide. Coder l'interface
de lecture/validationfailclosed en son absence, tester sur données synthétiques
connues, documenter le gap. Les consommateurs de preuve réelle ne seront admis
qu'après intégration du code ET du lock, revue/guards utiles fermés. Si un champ
de lock nécessaire n'est pas fixé par le contrat, proposer sa forme fermée dans
le plan avant implémentation ; le lead fournira les valeursobservées.

## Interface convenue et invariants

- PowerShellWindows5.1 natif, pas dépendancepwsh ajoutée. Interface
  `-Mode tools|browser|validate|run|selftest`, `-Output` et `-Config` quand requis,
  cheminsWindows absolus privés horsGit ; pas commande/exécutable/argv libre.
  Variablesbinaires `K6_WINDOWS_EXE`, `K6_BROWSER_EXECUTABLE_PATH` contrôlées.
- k62.3.0, CfT156.0.8078.73/win64, provenance/archive/exécutablesPE/x64/identités
  vérifiées **avant** exécution ; Chrome149 installé nonadopté. AssetsLinux non
  transposés, pas latest ni recherchePATH/fallbacksilencieux. Acquisition sépare.
- `tools` : versionsréelles et deuxinspect ; options exactes1VU/1itération/
  per-vu-iterations/délais15sHTTP/20sbrowser, pas seuildeperf. Aucune requêteapp.
- `browser` : JSexistant/about:blank/42/contexte neuf/fermeture. Sandbox actif,
  aucun no-sandbox/TLSignoré/UI/HAR/screenshot/debug/CDPexposé. Témoin effectif
  du renderer possédé selonChromiumWindows156 : restrictions du token/intégrité/
  AppContainer selonvariante documentée. JobObject/nom/absenceflags ne suffisent
  pas. Si preuve indisponible, rendre INCONCLUSIVE honnête, pas un PASS fabriqué.
- Parent : environnementallowlisté(K6/proxy/reportingdésactivé), capturesbornées
  privées et aucun fluxnatif ou exception libre publié. Projectionrésultatstrict
  lié au run courant, nombrecas/started/completed/iterations/checks exactsnonvides,
  résumésabsents/périmés/tronqués/metricsabsentes refusés. Sentinelles sur positifs
  etnégatifs ; fauteconfidentialité interditpublication etPASS.
- Processus : garderhandle créé, PID/création/SID/image réellementobservés avantfin,
  preuvefin par mêmehandle. PasrelectureSIDaprèsfin inventée ni PIDseul/nomadopté.
  Descendants possédés suivis sansfenêtre d'échappement ; JobObject affecté avant
  exécution sichoisi, nestingcompatible. Pas KILL_ON_JOB_CLOSE/TerminateJobObject/
  taskkill/killforcé. Fin normale puis arrêtgracieux borné des seulspossédés,
  jonction/inventaires prépost mêmeéchec/timeout/interruption. Inconnu nonvert.
- Gardes chemins/ACL/ancêtres/reparse/identitésWindows, pasadaptationdesdroitsdu
  disqueI: ni réemploi de la collecteprivée. Pasélevationdebugpour lireuntoken.
- HTTP seulementorigineJSfixe `http://127.0.0.1:18080`, cibleexistanteautorisée/
  ancrée/joignable. Aucun serveur factice/stack/tunnel/URLlibre/tokenexemption.
  Si aucuneancre conformen'est fournie, failclosed HTTP/selftestréelINCONCLUSIVE,
  zérorequête. Pas de nouveau mécanismed'autorité. `validate` readonly/sansGET.
  Selftest si cible admise : fauxcheck, inputabsent, cibleprérefusée, casabsent,
  interruption/résuméabsent ; chaque négatif nonzéro et agrégationexplicite.

## Méthode, QA et preuves

Lire sourcesofficielles versionnées référencéesdanscontrat avant choixtoken/job/
processus. Confirmer localement formesAPI/flags avecdocumentationWindows ; le
contrat n'est pas une preuve decompatibilité des primitives. Planifier puis
choisirreasoning dansenveloppenative, sans modèle explicite.

TestsWindows ciblés couvrant les **vrais callsites** de l'adaptateur/interop,
pinsversions négatifs, options/summariesnonvacants, environnement/confidentialité,
identité/descendant/cleanup, interruption/timeout/handle indisponible. Tests
synthétiques connus et petits enfantsPythonWindows de fixture autorisés sous
inventaire/bail propre ; pas vrais k6/Chrome/HTTP par auteur. Aucun runtime tiers.
Préserver ROUGEavantfix quandun test démontre unbug ; une panneprécondition n'est
pasROUGEproduit. `git diff --check`, suitesciblées seulement ; les194testsanciens
et27fixturesWSL restent àleurportée, pas rerun pourremplir unrapport.

Le lead réserve le slotCargo unique et les campagnes réelles. Aucune ressource
runtime créée ou redémarrée par ce worker horsfixtures connuesWindows. Rendre
inventairefactuel etcleanup des fixtures ; pasresourcename commepropriétaire.

## Livraison et frontières ouvertes

Livrer delta intégré dans votreworktree et rapportnatifcomplet, fichiers/blobs/
SHA256, commandes réelles/totaux/exit/skips, décisionslocales et limites. Pas
ledgerpartagé : le lead le persiste. ConsommateursQA/runs réels attendent arrêt,
intégrationnative ou récupérationbyte-vérifiée au lead et contrôle dudiff.
Ne pas chercher des fichiers publicsFATAL ou de nouvelles instrumentations pour
verdirun critère. Une preuve manquante resteINCONCLUSIVE avec repriseprécise.

Après deux tentativessubstantielles infructueuses sans progrès vérifié, remonter
aulead registre/hypothèse/expériencediscriminante, pas troisièmeboucle aveugle ou
cascade d'agents. Toute questionstructurelle nouvelle remonte àarchitecte via
lead ; détailslocaux compatibles et bugsordinaires restent àvotre main.
