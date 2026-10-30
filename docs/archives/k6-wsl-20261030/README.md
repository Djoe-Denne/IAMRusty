# Archives de l'enquête k6/Chromium sous WSL — 30 octobre2026

**Archive inactive. WSL : INCONCLUSIVE.** L'utilisateur demande de suspendre
cette approche et de tester sousWindows. La qualificationLinux sur une machine
GitHubActions sera une mission distincte. Ne lancer aucun outil de cette archive,
ne réveiller aucunruntime et ne reprendre aucune mesure à partir de ces fichiers.

## Ce qui est conservé

Copies textuelles byte-identiques de sources publiques ponctuelles utiles,
nommées avec le suffixe`.txt` pour ne pas être des scripts admis au lancement.
Elles sont placées hors`ops/tests/k6` et hors découverte des tests/CI. Le manifeste
indique l'origine, la taille et l'empreinteSHA256 ; il ne constitue ni un bail,
ni une permission, ni une preuve de bonne exécution des programmes.

Le manifeste initial contient19sources, copiées et vérifiées byte-identiques
sans exécuter aucun script archivé. Une règlelocaleGit désactive la conversion
des fins de ligne pour ces copies, afin de conserver leurs empreintes. Le
raccordldd-only éventuellement rendu au moment du gel sera conservé séparément
comme travail non consommé, jamais ajouté à la preuve du run historique.

La référence exécutée du diagnostic et la correction ultérieure du lecteur sont
conservées séparément. Cette correction n'a jamais requalifié le run réel et ne
doit pas servir à inventer sa sortie effacée. Les contrôleurs contiennent des
chemins ponctuels et des identités de campagne historiques : aucune portabilité
ou réutilisation directe n'est promise. Un usage futur exige un nouveau mandat
et une revue des préconditions, pas un simple changement de chemin.

Aucun journal natif, FATAL inconnu, profil navigateur, configuration privée,
relevé de site, bail, registre d'exécution, cache, binaire, image ouVHDX n'est
inclus. Les données/outils installés dans l'instanceAIForAll surI: sont conservés
séparément, sans copie dansGit. Les sauvegardes locales et les preuves privées
restent à leur emplacement de récupération ; leur conservation n'est pas une
publication ni un nettoyage terminé des worktrees.

## Résultats à leur portée exacte

- OutilsLinux : versions et inspections précédemment vérifiées, pas un PASSChrome.
- Isolationuserns : une commandeunshare réussie dans le lanceur supervisé ;
  aucune preuve de tous les mécanismes du sandboxChrome.
- Dépendances : une commandeldd, code0, nombre de bibliothèques manquantesinconnu.
  Le défaut de lecture du chargeur a été corrigé sur des fixtures connues.
- Navigateur : dernier cas réel non commencé, arrêtfatal observé, causeinconnue.
  Aucun nouvelessaiChrome durant cette enquête. QualificationWSLINCONCLUSIVE.
- Nettoyage : baux de sondes libérés, copies retirées, instanceAIForAll observée
  arrêtée après les opérations ; ressources étrangères inchangées. Cela ne vaut
  pas inventaire frais ou arrêt global deDocker/WSL.

Les sources durablesLinux sous`ops/tests/k6` et leurs tests ne sont pas déplacés
ou rebaptisésWindows. Elles restent un socleLinux historique pour une mission
future, avec leurs propres limites. Le conducteur privé et sa collecte ne sont
pas admis sous prétexte de ces archives.

## Références durables

- [État et suspensionWSL](../../wsl-aiforall.md).
- [Décision utilisateur](../../../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/76-arret-wsl-et-preparation-des-tests-windows.md).
- [Mesures réelles](../../../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/71-userns-reussit-dependances-encore-inconnues.md).
- [Défaut de lecture démontré](../../../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/72-chargeur-direct-et-defaut-du-lecteur-demontre.md).
- [Correction et27tests, pas qualificationChrome](../../../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/73-lecteur-corrige-et-27-tests-valides.md).

Les propositions Windowsaprès-exit et stockage non retenues restent explicitement
Proposed ; leur conservation documentaire ne les transforme pas en décisions.
