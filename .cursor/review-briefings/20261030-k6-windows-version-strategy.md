# Déterminer la version effective de Chrome Windows sans lancement UI

## Problème précis et mandat

Consultationarchitecte12 existant, mêmeprofil/modèle, documentaire uniquement.
Révisionnative2 Windowsapprouvée/hash4d651a… ; contratWindowsAccepted intégré
dans2cc7ae6, workflowLinuxsuspendulocalementabe170a. Aucune réouvertureWSL/ldd,
Linux/GitHubActions ou frontière de collecteprivée. Pasnouvelleautorisation de
runtime, codeOPS, scénario, mode/framework ou écart de politique par ce brief.

Le candidatWindows natif20, rapport `result_mvvbg53s_0000_8vtrli`, signale que
Chromium156 appelle `HandleVersionSwitches` sousIS_POSIX dans le source consulté.
Il n'a identifié aucun équivalentWindows ; son adaptateur refuse donc
CHROMIUM_VERSION_UNAVAILABLE avant tools/browser. AucunecommandeChrome--version
n'a été exécutée : ne pas la lancer pour « voir » si elle ouvre uneUI. Vérifier
ce constat ancré, pas l'accepter comme impossibilité absolue non recherchée.

Objectif : décider le moyen **minimal** et honnête de prouver la version du
ChromeWindows156 effectivement utilisé, sans affaiblir sandbox/confidentialité,
exposerCDP/debug, installerunoutil ou substituerVersionInfoàuneversionréelle.
L'exit/source/metadata/pin et une versionréelle observée restent distincts.

## Faits et surfaces disponibles

- ArchivesofficiellesWindows acquises et rehashées, PE64 et cheminszip vérifiés,
  puis exportportable306fichiers/huitexes vérifiés horsGit. Aucun k6/Chromeexécuté.
- k6v2.3.0, Chrome156.0.8078.73/win64/r1706539 ; Googlemetadata HTTPScorrélée mais
  SHAChromeobservélocal, pas checksuméditeurpubliéinventé. Chrome149installénonadopté.
- TroisJSactuelsinchangés, casbrowser33lignes about:blank/42/contexte neuf/counters1,
  wait750ms ; summary.js ferme run_id/phase/cases/cardinalités/checks, sansversion.
  L'origineHTTPreste18080literal et aucunecible admiseactuellement.
- Transportnormalinterne k6/Chromium autorisé par contrat ; un portCDP/debugexposé,
  loggerFATAL/sourceinconnu ou autrelanceur/superviseur n'est pas autorisé.
- Auteur20 dispose de codeprimitivesWindows et21testsrapportés (pasencorevérifiés
  parlead), rendererwitness implémenté/nonqualifiéréel, tousgapsontétérendus.

## Enquête et décision attendues

Privilégier sourcesofficielles k6v2.3.0/Chromium156 et documentationWindows.
Examiner si une APIexistante k6/browser fournit déjà la version effective (par
exemple Browser.version si réellement exposée), ou si le transportnormal lit
Browser.getVersion sans avoir à créer un autrecanal. Vérifier exports APIréels,
pas une simple méthodePlaywright ou deGo non exposée àJS. Si une preuveenmémoire
dans l'unique casabout:blank est possible, borner la projection et expliquer
comment les pins/metadata avantlancement puis versionréelle danslemêmecas sont
liés. Zéro requêteapplicative, UI ou deuxièmeessainavigateur ajoutéparfacilité.

Évaluer toute alternativecompatible documentée du binaireWindows lui-même sans
la lancer. Écarter VersionInfo/constante/env/métadonnéeseul comme équivalent de
versiond'unprocessusréel, Chrome149, nouveauChrome ou extensiondecontrôleslibre.
Pas de dumpDOMsurautreURL, chrome://version, serveur/outillage supplémentaire,
CDPpublic, désactivationdel'isolation/TLS, accèsadmin ou manipulationduprofil.

Si un ajustement domaineminimal est nécessaire, écrire seulement un nouveau
`docs/contracts/k6-windows-browser-version.md` **Proposed**, lié auWindowsAccepted,
avec décision/sources/versionnement/oraclesetlimites. Ne pas modifier Accepted,
OPS, policyJSON, ledger ou scripts. Le lead ratifiera danslemandat si compatible,
avant correctionconsommatrice. Un changement indispensable des commandesQA
approuvées doit être signalé explicitement, sans révision silencieuse ni GO user
catégoriel si les commandes sont simplement nonexécutables et une preuve admise
parlecritère peut être fournie autrement. Aucun contratQA ne peut être contourné.

Rendre un avis borné avec Localisation/Impact/Limites et corpsnatif complet+SHA
si document. Pas de délégation, tests, acquisition, commandeWindows/WSL/Cargo,
processus, navigateur ou sourceprivée. Pas de conclusiondecompatibilité réelle
avant une preuvefuturegardée ; difficultétechnique seule ne justifie pas pause.
