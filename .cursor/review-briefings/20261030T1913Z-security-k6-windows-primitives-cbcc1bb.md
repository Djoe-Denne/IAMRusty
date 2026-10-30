---
reviewer: security
written_at: 2026-10-30T19:13Z
branch: master
head_sha: cbcc1bbcf79d08f674e9274e2c63b864af086eb9
reviewed_commit: 68631c0f8ef296218e260d6bb44eb2abc5055a90
dirty: true
scope_slug: k6-windows-primitives
verdict: GO_WITH_COMMENTS
security_decision: REVIEW_REQUIRED
browser_admission: NOT_APPROVED
native_candidate: result_mvvcq3vd_0018_3enj9k
related_paths:
  - ops/tests/k6/lib/windows-native.cs
  - ops/tests/k6/lib/windows.ps1
  - ops/tests/k6/run-windows.ps1
  - ops/tests/k6/config/tools.windows.lock.json
  - ops/tests/k6/tests/test_runner_windows.py
---

## Verdict : REVIEW REQUIRED

**Un finding MEDIUM actif. Aucun CRITICAL/HIGH atteignable démontré dans `68631c0`.**
**Admission du navigateur non approuvée** : deux risques conditionnels doivent être traités ou revus avant de rendre son callsite accessible. Aucun code modifié, aucun test ni runtime lancé.

Lecture figée : `68631c0f8ef296218e260d6bb44eb2abc5055a90`. À 19:12 UTC, les trois sources Windows correspondaient encore exactement à ce commit. HEAD observé : `cbcc1bbcf79d08f674e9274e2c63b864af086eb9`.

## S-1 — Création non exclusive donnant une autorité de suppression sur un répertoire étranger

- **Severity:** MEDIUM
- **Confidence:** HIGH
- **Category:** race
- **Scope:** cross-file
- **Location:** `ops/tests/k6/lib/windows-native.cs:166–171`, `run-windows.ps1:36–37,92–95`, `lib/windows.ps1:175–190`
- **Security property violated:** intégrité des données ; suppression limitée aux ressources effectivement créées.
- **Attack surface:** chemin `-Output`, création et nettoyage.
- **Trust boundary:** chemin demandé → attribution de propriété → autorisation de suppression.
- **Attacker capability:** processus concurrent sous le même compte autorisé, capable de créer le chemin sous le parent privé. Ce finding ne suppose pas qu’un utilisateur extérieur puisse traverser les ACL vérifiées.

### Scénario

1. A constate que le chemin n’existe pas.
2. B crée ce répertoire et y dépose ses propres données.
3. `Directory.CreateDirectory(path, acl)` appelé par A accepte le répertoire désormais existant : cette API n’est pas une création exclusive.
4. A positionne néanmoins `$outputCreated = $true`.
5. Sur échec précoce, `$state.Cleanup` peut encore être vrai : A appelle alors `Remove-K6OwnedTree` sur le répertoire créé par B.

Même si un handle de B empêche la suppression finale du répertoire, la boucle de nettoyage peut avoir déjà supprimé des fichiers enfants fermés.

**Evidence:** le contrôle préalable `Directory.Exists/File.Exists` est séparé de la création ; aucun résultat atomique « créé par cet appel » n’est conservé. Le drapeau de suppression est positionné avant la construction réussie d’`$outputPin`.

**Impact:** suppression ou altération de données d’un autre run ou d’une autre opération locale. Impact contraint par le parent privé et la concurrence nécessaire.

**Recommended fix:** création native atomique du seul répertoire final, avec DACL fournie à la création ; refuser `ERROR_ALREADY_EXISTS` sans mutation. N’accorder l’autorité de nettoyage qu’après création effectivement réussie et liaison à l’identité observée. Un échec de cette liaison ne doit jamais autoriser le nettoyage d’un gagnant concurrent.

**How to validate / Regression test:** dans une fixture jetable, synchroniser deux créateurs autour de cette frontière. Le perdant doit refuser sans écrire, parcourir ni supprimer le répertoire du gagnant ; son fichier témoin doit rester intact. Non exécuté ici.

**CWE:** CWE-367.

## Réserves avant activation du navigateur

Ces chemins sont **inaccessibles depuis l’entrée actuelle** : `run-windows.ps1:66` appelle la fonction qui refuse systématiquement la preuve de version, avant le lancement navigateur ligne 71. Ils ne constituent donc pas des HIGH exploitables aujourd’hui. Cette barrière ne doit pas être retirée en considérant la présente revue comme une approbation du navigateur.

### R-1 — Les pins ne couvrent pas le code DLL effectivement chargé

**Localisation :** `lib/windows.ps1:90–111,134–136`, `windows-native.cs:266–278`.

`Open-K6PinnedTool` vérifie et conserve l’archive et `chrome.exe`, mais ni l’intégrité ni les ACL ni les identités des DLL extraites.

Les sources officielles Chromium **156.0.8078.73** confirment que :

- `chrome_exe_main_win.cc:376–379` charge le module principal ;
- `main_dll_loader_win.cc:117–121,192…,243–244` sélectionne notamment `chrome.dll`/`chrome_renderer.dll` et appelle `LoadLibraryExW`.

**Chemin d’abus conditionnel :**

- un acteur moins privilégié possède seulement un droit d’écriture sur une DLL préexistante, ou l’extraction contient une DLL altérée ;
- l’archive et `chrome.exe` restent authentiques et inchangés ;
- les contrôles actuels réussissent, puis le véritable exécutable charge cette DLL sous le compte opérateur.

Une ACL restrictive du répertoire parent ne valide pas les ACL individuelles des fichiers préexistants et ne relie pas leurs octets à l’archive. Le chemin d’image observé et une version runtime correcte ne détectent pas cette substitution.

**Impact conditionnel : HIGH — exécution sous le compte opérateur.** Si le raccord de version rend ce chemin accessible sans correction, la revue de ce delta doit être **BLOCK**, pas une acceptation implicite du risque.

**Correction attendue :** établir une liste fermée du code livré effectivement chargeable, vérifier ses octets contre l’archive authentifiée ou un manifeste revu, puis conserver les garanties d’identité, de non-reparse, d’ACL et de non-modification jusqu’à la fin des processus concernés.

**Validation future non destructive :** modifier uniquement une dépendance factice jamais exécutée et vérifier que le vrai préflight refuse avant toute création de processus. Ne pas fabriquer ni charger une DLL hostile.

### R-2 — Le témoin du jeton primaire ne prouve pas la restriction effective complète

**Localisation :** `windows-native.cs:91–104,272–278`.

Deux limites démontrées :

1. `RendererToken` ne lit que le **jeton primaire**. La documentation Chromium versionnée explique explicitement qu’au démarrage le processus possède déjà son jeton primaire lockdown, alors que le thread initial conserve un jeton d’impersonation plus puissant jusqu’à `LowerToken()`.
2. Le booléen `Sandbox` devient définitivement vrai après **un seul** échantillon positif de n’importe quel renderer observé. Il n’est pas lié à une observation post-initialisation du renderer ayant exécuté le cas.

Le prédicat vérifie aussi seulement la présence du SID restreint nul et l’absence de privilèges **activés** ; il ne vérifie pas les groupes ordinaires attendus ni l’absence des privilèges documentée pour le token lockdown.

**Conséquence :** ces faits prouvent certaines restrictions du jeton primaire échantillonné, pas la fin effective du bootstrap ni l’intégralité de la politique revendiquée. Aucune évasion réelle n’est affirmée par cette revue statique.

**Correction attendue :** ne pas transformer ces observations seules en qualification sandbox PASS. Exiger une preuve bornée et documentée correspondant à l’état effectif attendu ; si elle est inaccessible avec les droits ordinaires, conserver **INCONCLUSIVE**, sans élévation ni instrumentation intrusive ajoutée implicitement.

**Régression future :** un jeton primaire restrictif avec jeton initial encore effectif, ou un témoin provenant seulement d’un autre renderer, ne doit pas suffire à qualifier le cas.

## Passes effectuées

### PASS A — Défensive

Contrôles effectivement présents :

- création assignée au Job Object avant exécution ; handles conservés ;
- absence de kill-on-close et de terminaison forcée implicite ;
- contrôle d’appartenance au job avant adoption des descendants ;
- captures bornées, overflow persistant et décodage UTF-8 strict ;
- échec fermé lorsque identité, capture ou cleanup restent incomplets ;
- environnement enfant reconstruit, sans héritage K6/proxy/loader ;
- projection publique fermée, sans restitution des flux ou exceptions natifs ;
- résumé lié au run, cardinalités et checks exacts ;
- HTTP refusé avant exécution faute d’autorité Windows admissible.

Les garanties de création exclusive, d’intégrité du code DLL et de témoin sandbox effectif présentent les limites décrites ci-dessus.

### PASS B — Adversariale

Chaînes examinées :

- course de création → adoption incorrecte → nettoyage étranger ;
- archive/exécutable authentiques + DLL altérée → chargement sous l’opérateur malgré les pins ;
- restrictions primaires observées tôt → témoin persistant → qualification plus forte que les faits observés.

Pas de chemin démontré d’exposition de secret par la projection fermée ni d’adoption d’un processus étranger sur son PID seul.

## Sources et empreintes examinées

SHA256 au commit figé :

- `windows-native.cs` : `82b00f61fabc697f2872605033eead985fbd49fdfb1005d34b70692461bdc2dc`
- `windows.ps1` : `7639bee526816e88d1366de02475aeac567504f0741a4ef44b0d8102748e527d`
- `run-windows.ps1` : `c17a9ba8eedeffe4ed09779b7716ce1f5e512c19b514b65bd363bd3b1e09b378`
- lock Windows : `e1446138551b3dea5843bc19fb647233901c0d2f20beb23363a37cfd1b4e1b06`
- tests Windows : `eaf18f05e2b39b242b5fea551e84515110e849f92a9790b4496d67bc6d09255c`

Contrats fonctionnel/version Accepted, tests et scénarios locaux lus. Le changement documentaire ultérieur observé porte seulement sur `Unimplemented → Partial` ; ses résultats annoncés ne sont pas utilisés comme preuves de cette revue.

Sources officielles recoupées :

- [Chromium 156 — sandbox, token et bootstrap](https://github.com/chromium/chromium/blob/156.0.8078.73/docs/design/sandbox.md), sections lignes 136–157 et 554–599.
- [Chromium 156 — entrée Windows](https://github.com/chromium/chromium/blob/156.0.8078.73/chrome/app/chrome_exe_main_win.cc).
- [Chromium 156 — chargement DLL](https://github.com/chromium/chromium/blob/156.0.8078.73/chrome/app/main_dll_loader_win.cc).
- [Microsoft Reference Source — Directory.CreateDirectory](https://github.com/microsoft/referencesource/blob/main/mscorlib/system/io/directory.cs), notamment traitement concurrent d’`ERROR_ALREADY_EXISTS`. Référence sémantique, pas preuve de version de l’assembly hôte.

## Vérifications et limites

- `git diff --check` : **exit 0**.
- `git diff --check 68631c0^ 68631c0` : **exit 0**.
- Suite unittest Windows : **NOT_EXECUTED**, interdite dans cette tâche.
- Versions k6 et Chrome : **NOT_EXECUTED**.
- Les deux `k6 inspect` : **NOT_EXECUTED**.
- Modes `tools`, `browser`, `validate`, `run`, `selftest` : **NOT_EXECUTED**.
- Aucun HTTP, Chrome, WSL, Docker, Kind, Cargo, installation ou changement de permissions.
- Aucun PASS fonctionnel, sandbox ou cleanup réel déduit de cette analyse.
- Aucun avis sur les modifications de version postérieures au snapshot.

## Continuité

**Do not redo :** ne pas relancer les suites historiques ou un probe WSL ; réutiliser les constats et sources ci-dessus.
**Anti-goals :** aucune activation browser, élévation, relaxation des gardes ou modification de contrat implicite.
**Staleness :** toute levée du refus de version exige une revue du delta intégré et des réserves R-1/R-2. Persister ce briefing et son entrée INDEX avant correction.

Note du lead : le champ historique `verdict: GO_WITH_COMMENTS` est conservé tel que
fourni ; il ne vaut pas admission. La décision opérante est REVIEW_REQUIRED et
browser_admission NOT_APPROVED. Le lancement réel et la qualification restent gelés
jusqu'à disposition vérifiée des réserves concernées.
