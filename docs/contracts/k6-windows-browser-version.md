# Version effective Chromium Windows via l'API normale k6

- Statut : **Accepted** ; Réalité : **Implemented et vérifiée le 2026-10-31** :
  check API obligatoire dans le même cas Windows, version156.0.8078.73 égale au pin,
  cleanup/confidentialitéPASS. [Preuve du cas minimal](../../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/99-cas-minimal-windows-verifie.md).
  Chrome `--version` reste NOT_EXECUTED/INCONCLUSIVE ; aucune qualification Linux
  ou attestation du sandbox n'est déduite de ce résultat fonctionnel.
- Date : 2026-10-30 ; mission1, Windows seulement ; lié au
  [contrat Windows Accepted](k6-windows-functional.md), ratifié dans2cc7ae6.
- Entrées : briefing `20261030-k6-windows-version-strategy.md`, révision2 approuvée
  `4d651a945ab4f6757f24117ed715c37cf82fc712bf5b22721d593240543cac64`,
  critères108/110 et commandes du `model-policy.json` canonique lus sans modification.
  Candidat20 partiel et21tests sont rapportés, pas vérifiés ou qualifiés par cet avis.

## Ratification et raccord de preuve

Ratifié par le lead le 2026-10-30 dans le mandat long run de la révision2.
Corps architecte reçu et comparé byte-identique avant ratification : SHA256
`7af3137d6db8b07d125ba5ee891ec2254298a5d27efa1134815f5fd434952b37`.
Le lead a recoupé les sources versionnées du mapping, de Browser.Version/GetVersion
et du garde IS_POSIX. Ce choix minimal reste dans l'unique cas about:blank autorisé.

La liste QA approuvée demande de rapporter le résultat réel des commandes, pas de
lancer un flag non portable malgré l'interdiction d'interface graphique. La commande
Chrome `--version` reste explicitement **NOT_EXECUTED / INCONCLUSIVE**, jamais PASS.
Son oracle de version effective est admis via le check API du même cas gardé.
La politique native et sa liste de commandes ne sont ni modifiées ni présentées
comme toutes exécutées. Aucun résultat global ne masque cette non-exécution.

`tools` seul conserve tools=INCONCLUSIVE et exit2 tant que la version Chrome n'a
pas été observée. Dans le mode browser, les préconditions et inspections peuvent
libérer ce seul cas ; tools ne devient PASS qu'après le check courant de version
et les autres preuves requises. Un échec de cleanup/confidentialité invalide le
résultat positif. Pas de deuxième lancement pour prouver la version.

Cette précision amende uniquement la conservation fonctionnelle du cas navigateur
du contrat Windows : ajout conditionnel du check version, obligatoire sous Windows.
HTTP et summary.js sont inchangés, Linux suspendu et non qualifié. Les formulations
propositionnelles suivantes conservent la provenance de l'avis, pas une attente
de nouvelle approbation humaine de routine.

## Constat sourcé et décision

Chromium156 appelle HandleVersionSwitches sous IS_POSIX, hors renderer build ;
cette voie couvre --version/--product-version, pas un mode Windows non-UI garanti.
L'entrée Windows ciblée ne fournit pas de preuve alternative dans les sources lues.
Cela justifie de ne pas essayer ces flags ; pas une impossibilité universelle alléguée.

**Retenir `browser.version()` dans l'unique cas navigateur existant.** L'API est
réellement exposée par le mapping JS de k6v2.3.0 et reliée au module browser géré,
pas une méthode Playwright supposée ou une méthode Go inaccessible. Elle renvoie
une string de façon synchrone. common.Browser.Version retire le préfixe produit
avant le premier slash ; la valeur provient de Browser.getVersion exécuté par k6
sur sa connexion normale à l'initialisation, puis **mis en cache**. Ce n'est ni
VersionInfo ni une interrogation fraîche répétée à chaque appel de version().

Pas de nouveau navigateur, cas, superviseur, mode, client/port CDP ou logging.
Aucun chrome://version, dumpDOM, UI, UA/page falsifiable utilisé comme preuve,
Chrome149, changement de pin ou sandbox/TLS relâché. Le transport interne de k6
déjà admis suffit ; aucun accès aux connexions distantes ou navigateur non possédé.

## Ajustement consommateur minimal proposé

Après ratification du lead, permettre seulement une lecture en mémoire et un
check Windows obligatoire `browser-version` dans browser-prerequisite.js existant,
à l'intérieur du try/finally du contexte. Cas/executor/VU1/itération1, about:blank,
évaluation42, compteurs, délais et fermetures restent identiques. C'est une précision
explicite à « JS sans modification fonctionnelle », pas une édition Accepted ici.

Le parent injecte un attendu public fermé `K6_EXPECTED_BROWSER_VERSION`, lié au
lock revérifié, exactement156.0.8078.73 pour cette révision. Ni valeur libre de
campagne ni héritage ambiant ; sa présence/type/valeur sont requis avant lancement.
La string réelle de browser.version() doit être bornée (32caractères maximum),
de forme stricte quatre entiers décimaux et égale à cet attendu. Absente, inconnue,
mal formée, API indisponible ou différente => check non réussi/observation non prouvée,
jamais constante de remplacement. Aucun texte retourné ou message natif n'est loggé.

Le check peut être conditionné à cet attendu pour préserver l'ancien usage Linux
non exécuté ; en Windows il n'est jamais facultatif : le parent exige exactement
un pass/zéro fail de `browser-version`. Son absence ou zéro exécution ne passe pas.
Tester explicitement ce cas, sans réactiver Linux ou modifier le scénario HTTP.

Réutiliser la projection fermée des checks de summary.js, sans nouveau champ de
flux/version libre ni métrique brute. Le parent ne peut attester l'égalité au pin
qu'avec ce check courant exécuté, les cardinalités exactes et les sources/identités
liées au même run navigateur. Noter la provenance « égalité observée via API k6,
cache Browser.getVersion à la connexion » ; l'attendu seul n'est pas une observation.
Si le parent projette la version égale au pin, qualifier cette inférence d'égalité,
pas de lecture indépendante ou de nouvelle interrogation CDP après le check.

## Préconditions, versions et ordre des phases

Avant le seul lancement navigateur : provenance, archives/exécutables PE/hash,
chemins/identités et environnement Windows contrôlés ; k6version et inspections
gardés comme prévu. Ces preuves statiques rendent le binaire éligible à la phase
gardée, sans prouver sa version effective d'exécution.

Séparer l'éligibilité de lancement de la qualification : tools peut constater
les préconditions sans annoncer Chrome-version PASS. La version effective reste
en attente de l'unique cas browser, pas remplie depuis metadata et pas un faux
échec irréparable interdisant la seule phase capable de la prouver. Le sens exact
du résultat/exit tools doit rester explicite : précontrôles seuls ne valent pas
qualification complète. Aucun gate de hash, identité, confidentialité ou processus
n'est supprimé ; seuls l'appel CLI non portable et sa dépendance circulaire sont visés.

Après la preuve API, relier outils et browser par pins/identités revérifiés, source
et run/campagne ; changement de binaire ou résultat ancien => preuve non réutilisable.
Startup échoué avant le cas => aucune version runtime observée, INCONCLUSIVE.
Version établie mais témoin sandbox/retrait absent => Chromium non qualifié. Les
contrôles Windows/renderer, sorties, jonctions, retraits et HTTP conditionnel restent
ceux du contrat Accepted ; aucun FATAL privé ou nouvel essai ajouté pour la version.

## Contradiction QA à traiter explicitement

La révision approuvée contient `& $env:K6_BROWSER_EXECUTABLE_PATH --version`
(verification.commands, ligne95 du fichier lu). Cette commande n'est pas exécutée
par cet avis ni remplacée silencieusement par VersionInfo. Son résultat reste
NON_EXECUTEE / INCONCLUSIVE pour non-portabilité étayée, jamais PASS fabriqué.

Le critère108 exige la version du binaire réellement utilisé et110 les versions
réelles, sans imposer cette primitive CLI. La preuve API dans le même cas est une
recommandation compatible avec ces critères. Le lead doit consigner et admettre
explicitement le raccord commande→preuve et l'ordre tools/browser avant correction.
Si le contrat QA natif impose l'exécution littérale de chaque commande, sa révision
précise reste nécessaire : la présente proposition ne le dispense pas et ne modifie
aucun QAJSON. Pas de GO utilisateur catégoriel pour une simple ratification de domaine,
mais aucun check obligatoire déclaré satisfait sans preuve admise par sa règle QA.

## Validation future et portée

Tests ciblés des vrais callsites : API absente/exception, valeur exacte,149/autre
version, chaîne vide/mal formée/sentinelle, attendu absent/modifié, check manquant
ou non exécuté, résumé périmé, binaire substitué et cleanup manquant. Une valeur
simulée incorrecte doit rendre le check faux malgré un attendu correct. Aucun
résultat de fixture ne devient preuve de version effective de l'exécutable.

Impact futur borné : adaptateur run-windows.ps1, un check du cas existant et tests
Windows correspondants ; summary.js réutilisé. Pas de nouvelle ADR : précision
diagnostique réversible dans la frontière déjà ratifiée. Aucun OPS/Accepted/QAJSON/
ledger modifié ici, aucun test, navigateur ou commande Windows/WSL exécuté.

## Sources versionnées vérifiées

- [Chromium156 chrome_main_delegate.cc:466–487,1255–1267](https://github.com/chromium/chromium/blob/156.0.8078.73/chrome/app/chrome_main_delegate.cc),
  [entrée Windows ciblée](https://github.com/chromium/chromium/blob/156.0.8078.73/chrome/app/chrome_exe_main_win.cc).
- [k6v2.3.0 module.go:96–109](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/browser/module.go),
  [mapping JS:73–79](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/browser/browser_mapping.go).
- [k6v2.3.0 common/browser.go:90–99,787–807](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser.go) :
  connexion, cache et Browser.getVersion ; aucune de ces sources ne prouve un run Windows réussi.
