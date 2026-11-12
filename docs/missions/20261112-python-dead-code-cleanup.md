# Nettoyage Python conservateur — 12 novembre 2026

Base : `54a19bae5ac270e0ed71b2e17c49c863dbfe3f75`. Exécution directe,
sans agent secondaire ni opération sur le lab. Les changements Obsidian
préexistants sont hors périmètre d'écriture.

## Résultat

Une seule suppression retenue : la constante globale `REDIRECTS` dans
`ops/tests/support/oauth-idp-simulator.py`. Le contrôle effectif utilise
`Provider.redirects`, construit pour l'instance et l'origine autorisées.
Aucun lecteur de la constante trouvé dans les sources/lanceurs/tests courants.
Les occurrences des anciennes suites sous `.tmp/iv-baseline` et
`.tmp/kind-minimal-before-20261109T190024/files` chargent leur propre copie
archivée du simulateur, pas le fichier courant. Archives conservées.

**Aucun fichier Python entier déclaré supprimable.** L'état actuel est
satisfaisant pour cette passe ; retirer le legacy restant serait un chantier
explicite de retrait de fonctionnalités, pas une suppression automatique de
code prouvé mort.

## Méthode et candidats conservés

- Inventaire : 38 fichiers Python suivis, dont 34 sous `ops/` (9 d'exécution,
  25 de tests). Les quatre autres, sous skills/Obsidian, sont examinés sans édition.
- Vulture 2.16 isolé via `uv tool run`, sans installation globale : 30 signaux
  runtime, 17 avec les tests avant nettoyage, 16 après. Les quatre autres
  fichiers ne produisent aucun signal. Ces nombres ne mesurent pas le code mort.
  Exit 3 signifie signalements ; les diagnostics de parsing doivent aussi être lus.
- Recoupement Serena, recherches exactes, lanceurs PowerShell, workflow k6,
  imports dynamiques, contrats, handoffs et scripts locaux hors Git.
- `negative_main` est appelé dans le driver Python textuel de `selftest` ;
  les callbacks HTTP et les paramètres `__exit__` sont protocolaires.
- `resume_nodes`, `stop_nodes`, `lifecycle_nodes`, `_power_node` restent :
  `.tmp/20261101-prove-local-power.py`,
  `.tmp/20261101-enable-existing-cri.py` et
  `.tmp/20261101-ext-authz-oci-preflight.py` appellent cette autorité. Ne pas
  assimiler l'absence d'appel dans les fichiers suivis à une absence locale.
- `registry_connect`/`registry_pull` et `_iam_secret_metadata` restent couverts
  par les tests/contrats, même sans consommateur runtime courant identifié.
  Les références Serena seules omettent certains de leurs usages de tests.
- `browser_observation`, `renderer_observed` et les autres attributs résiduels
  ne sont pas retirés : absence de lecture directe insuffisante pour exclure
  un consommateur dynamique ou un contrat de diagnostic. Ce ne sont pas des
  suppressions prouvées par cette passe.

## Validation et limites

- Simulateur : **9/9 PASS** ; k6 : **195 PASS, 8 POSIX non applicables**,
  203 tests au total. `git diff --check` réussi.
- L'édition symbolique de la constante a initialement laissé son expression
  sans nom : erreur de syntaxe détectée par les tests, corrigée par patch borné.
  Les résultats ci-dessus sont ceux du contenu final, pas de cet état intermédiaire.
- Pas de nouvelle campagne navigateur/Kind : la preuve k6 existante reste celle
  du commit **54a19ba**, run `e3337dff24eb470881233fdfad6f6022`. Le simulateur
  ayant changé d'empreinte, ne pas annoncer le nouveau tree qualifié en E2E.
- Lab, CA et runtime Docker/WSL non manipulés ; aucune ressource Docker créée,
  donc aucune nouvelle entrée de création au registre. Aucun nettoyage d'archives.
- Une analyse statique et les tests ne prouvent pas l'absence de consommateurs
  privés inconnus ni l'exhaustivité des chemins dynamiques. Pas de whitelist
  globale pour masquer les signaux restants, pas de CI ajoutée.

Preuves locales : `.tmp/python-dead-code/` (inventaire, scans avant/après,
sorties des tests). Outil et limites :
[Vulture 2.16](https://github.com/jendrikseipp/vulture/blob/v2.16/README.md).
