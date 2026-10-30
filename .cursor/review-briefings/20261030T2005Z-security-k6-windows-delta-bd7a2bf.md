---
reviewer: security
written_at: 2026-10-30T20:05Z
branch: master
head_sha: bd7a2bf58d7e079c5dec2fe141838255f29efa29
reviewed_base: 68631c0f8ef296218e260d6bb44eb2abc5055a90
dirty: true
scope_slug: k6-windows-delta
verdict: GO_WITH_COMMENTS
security_decision: REVIEW_REQUIRED
live_run_admission: FROZEN_R1_RETENTION_OPEN
native_candidate: result_mvvemxgb_001m_ynm8dy
related_paths:
  - ops/tests/k6/lib/windows-native.cs
  - ops/tests/k6/lib/windows.ps1
  - ops/tests/k6/run-windows.ps1
  - ops/tests/k6/scenarios/smoke/browser-prerequisite.js
  - ops/tests/k6/tests/test_security_windows.py
  - ops/tests/k6/tests/test_browser_version_windows.py
---

## Relecture ciblée — REVIEW REQUIRED

**S-1 fermé au niveau du code. R-1 préflight fermé, rétention résiduelle ouverte. R-2 faux PASS fermé ; qualification sandbox toujours INCONCLUSIVE.**

Aucun nouveau CRITICAL/HIGH démontré sur les fragments corrigés. **Le gel des runs réels reste nécessaire pour le fragment R-1 encore ouvert.** Cette réserve ne bloque pas les investigations indépendantes autorisées.

Source examinée : `68631c0..bd7a2bf58d7e079c5dec2fe141838255f29efa29`. HEAD encore identique en fin de revue ; aucun delta OPS supplémentaire observé. Aucun fichier modifié, test ou runtime lancé.

### S-1 — Fermé : création et autorité de nettoyage

**Localisation :** `windows-native.cs:199–260`, `run-windows.ps1:39–40,95–102`, `windows.ps1:330–350`.

Constats indépendants :

- `NtCreateFile` utilise `FILE_CREATE`, relativement au handle du parent vérifié, avec le descripteur privé fourni à la création.
- Le succès exige le handle retourné et `FILE_CREATED`. Un chemin existant ne devient plus une propriété adoptée.
- `$outputCreated` n’est positionné qu’après retour réussi de `OwnedDirectory.Create`, donc après binding.
- Le nettoyage exige un objet `OwnedDirectory` encore lié ; il vérifie l’identité avant tout parcours.
- Le handle d’origine reste ouvert sans partage DELETE. La suppression finale utilise `FileDispositionInfo` sur ce même handle, sans intervalle « libérer puis supprimer par nom ».

**PASS B :** concurrence de création, binding indisponible, objet disposé et remplacement du chemin n’accordent plus d’autorité de suppression sur le gagnant étranger.

Les fixtures lues ciblent ces frontières : créateurs synchronisés, refus de réutilisation, binding indisponible et préservation d’un remplacement. Leur exécution n’a pas été répétée ici.

Cette fermeture concerne la course initiale et l’autorité destructive ; elle ne constitue pas une preuve générale de cleanup sur tous les échecs.

### R-1 — Préflight fermé, durée de rétention toujours ouverte

#### Partie corrigée

**Localisation :** `windows.ps1:90–218,242–245`.

Le bundle n’est plus réputé authentique à partir du seul couple archive/exécutable :

- ensemble fermé dérivé du ZIP déjà authentifié ;
- refus des collisions insensibles à la casse, traversées, alias ambigus, entrées de lien/type incohérent et membres manquants ou supplémentaires ;
- comparaison des octets de chaque fichier extrait avec son entrée authentifiée ;
- vérification individuelle des chemins, ACL et identités ; handles empêchant écriture et substitution conservés ;
- limites explicites : 4 096 entrées, 512 MiB par fichier, 2 GiB d’expansion déclarée, comptage des octets effectivement décompressés ;
- exigence des leases au vrai callsite `Invoke-K6Child`.

**PASS B :** modifier une DLL en laissant archive et exécutable intacts ne contourne plus le préflight. Les fixtures utilisent des octets inertes et exercent aussi absence, ajout, collision, traversée et refus de réécriture pendant la rétention.

**Fermeture bornée :** l’attaque initiale sur une dépendance déjà altérée est fermée pendant la période de custody effective des leases.

#### Fragment restant à revoir — OPEN

**Localisation :**

- `windows-native.cs:388–432` : `Finish` peut revenir avec `Cleanup=false`, puis `Child.Dispose` libère les handles ;
- `windows.ps1:264–272` : jonction bornée puis disposition du `Child` ;
- `run-windows.ps1:94` : `Close-K6PinnedTool` est exécuté même lorsque le cleanup reste non prouvé.

Le parent peut donc rendre la main avec un survivant ou une absence non prouvée, puis libérer les protections des fichiers susceptibles d’être chargés ultérieurement. La disparition du processus hôte libère également ces handles.

Les verdicts non positifs et la préservation du répertoire privé **ne prolongent pas** ces protections. La documentation signale correctement cette limite ; elle ne la corrige pas.

**Condition de fermeture :** revoir la disposition intégrée assurant la custody du code et des chemins pour toute la durée requise, y compris survivant et fin d’hôte. Aucune architecture nouvelle, attente indéfinie, terminaison forcée ou mutation d’ACL existante n’est autorisée par ce rapport.

**Validation future attendue :** cas contrôlé à durée naturelle bornée dépassant la jonction initiale ; vérifier la protection des dépendances jusqu’à la fin constatée, puis leur libération. Rien n’a été exécuté ici.

### R-2 — Faux PASS fermé, preuve effective non établie

**Localisation :** `windows.ps1:301–328`, `run-windows.ps1:78–79,103–106`.

- `Sandbox=true` et `BrowserArguments=true` ne peuvent plus qualifier Chromium.
- `Set-K6BrowserEvidence` conserve systématiquement `chromium=INCONCLUSIVE`.
- Le callsite produit alors `SANDBOX_UNPROVEN` et un exit non nul.
- La version peut être établie séparément par le check courant, sans transformer cette égalité en preuve sandbox.
- Un échec final de cleanup ou confidentialité révoque les verdicts positifs et la projection de version Chromium.

**PASS B :** un résumé valide accompagné des deux booléens positifs n’ouvre plus de voie Chromium PASS. Le gap du témoin primaire persiste explicitement, sans affirmation d’évasion ou de sandbox désactivé.

Le raccord version est également fermé sur les entrées examinées : attendu public exact, indépendant de l’environnement ambiant ; retour API typé, borné et égal au pin ; check obligatoire exactement une fois dans le résumé Windows. Son absence côté JS ne peut pas satisfaire le parent. Aucun texte natif de version n’est publié.

### Intégrité du périmètre

Comparaison directe effectuée :

- bloc `RendererToken` : **identique à `68631c0`** ;
- classe `Child`, incluant Job/observation/capture/jonction : **identique à `68631c0`** ;
- lock Windows, workflow, scénario HTTP et `summary.js` : **inchangés**.

Aucune nouvelle dépendance externe ni import distant ajouté dans le delta examiné.

### Empreintes vérifiées

Les dix fichiers correspondaient au worktree lors de la comparaison :

- `lib/windows-native.cs` — 36 648 octets
  `c653935f274a213559a27fb0c0f7e532c0736f254155d9d32f3a02be1ee801e5`
- `lib/windows.ps1` — 25 873 octets
  `5ba673483e8cf0d9281ebdc69d31f1f79cfc1a8838d736b354179589afa759b9`
- `run-windows.ps1` — 8 147 octets
  `e27a49320125a6353945d655352e54810f4bb48d129ff231222213b93289a455`
- `scenarios/smoke/browser-prerequisite.js` — 1 875 octets
  `aae10e84d8f7befa277d64c82644d1dfa88ad5a401535d49701be612aee557fb`
- `tests/test_security_windows.py` — 8 946 octets
  `4718e1ce6833e9b1d21cf403867b310cec57f0cf7f4ef1fa4578702f72cdb9ca`
- `tests/test_browser_version_windows.py` — 11 213 octets
  `2eb7d2f44aa0d6388066dbe631a2d25038b395a4f4ac9849202d9342b99ff899`
- `tests/test_runner_windows.py` — 24 339 octets
  `441c72eee22726260f541962e5eb1f4aa8779279847a645f0bf890aecd7f5dea`
- `IMPLEMENTATION.md` — 36 241 octets
  `2f49fcf17fb90178f33eb93333dd13df56f0fdf95b7a23dd2e4b614a254188e3`
- `README.md` — 42 084 octets
  `d7590446f32acaf70b5601dc68cc73a3750cf305929fdbf2a9d9b5f6e0690c1e`
- `WINDOWS.md` — 17 403 octets
  `d1d399288de543dbe337d758c2d1e937395ceb6cb483f5bde1d6b7f7080add0e`

### Sources et vérifications

Sémantique recoupée dans les références Microsoft de [NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile),
[SetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle)
et [FILE_DISPOSITION_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_disposition_info).
Les constats Chromium156 de la première revue restent applicables ; aucun nouveau témoin n’a été inventé.

**Exécuté par cette revue :**

- `git diff --check` : **exit 0**.
- `git diff --check 68631c0 bd7a2bf` : **exit 0**.
- Lectures, comparaisons et empreintes ci-dessus.

**Preuve communiquée par le lead, non réexécutée :** 22tests ciblés,23,986s,exit0,
zéroskip ; fixturesNode/Python terminées et dossiersretirés. Sources correspondantes
inspectées. Ce résultat ne vaut ni exécution de toute la commande unittestdiscover
ni qualification réelle des binaires.

Suite complèteWindows, versionsk6/Chrome, deuxinspections et modes tools/browser/
validate/run/selftest : NOT_EXECUTED par cette revue. AucunChrome/HTTP/WSL/Docker/
Kind/Cargo, installation ou changementdepermissions.

**Do not redo :** ne pas rouvrirS-1 ni refaire l’auditglobal pour la seule rétention ;
conserver les preuves du préflight et du refusChromiumPASS.
**Anti-goals :** aucun runréel ou nouveau mécanismedesupervision implicitement admis.
**Reprise précise :** disposition de rétention intégrée, puis revue du fragment
survivant/find’hôte et de ses vrais callsites. Aucun avis sur un futur diff inconnu.

## Acceptation de portée par le lead

Rapport substantif reçu après l'addendum, rapproché des dixempreintes de bd7a2bf et
des testslead22 déjà archivés. S-1 et le préflightR-1 sont fermés dans ce périmètre ;
le fauxPASSR-2 est fermé, pas la preuve d'isolation. R-1 rétention reste OPEN et le
gel d'exécution réel est conservé. Aucun résultat global ou admission de navigateur
n'est déduit du champ GO_WITH_COMMENTS conservé dans les métadonnées.
