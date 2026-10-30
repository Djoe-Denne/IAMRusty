# Fermeture ciblée — conducteur privé

```yaml
reviewer: security
written_at: 2026-10-30T15:42Z
branch: master
head_sha: 0f32e8dab504458074a353cb555139801db81487
dirty: true
scope_slug: k6-private-parent-closure
verdict: PASS
related_paths:
  - ops/tests/k6/local-private-parent.ps1
  - ops/tests/k6/local-private-parent.py
  - ops/tests/k6/tests/test_local_private_parent.py
```

## Verdict

**PASS limité à la fermeture de S-1 et S-2.** Aucun nouveau finding, aucune exemption. Le stockage réel reste non admis : cette fermeture ne constitue ni un préflight réel ni une autorisation de collecte.

Diff réel contre `195059a` examiné. Blobs vérifiés avant et après les tests :

- PowerShell : `27e3504e52a670ef9588e9ee38338aec86f8d516`
- Python : `c7aa95ed70079cb8e868d2dbd39024223523bea4`
- Tests : `b78e8093754033f8138564b5e56c1782e6a290e4`

Les autres sources outillage/workflow restent inchangées.

## Findings

Aucun nouveau finding dans le périmètre demandé.

### S-1 — FERMÉ

**Localisation :** `local-private-parent.ps1:193–268,432–434,476–484,510`.

La chaîne entière est observée : propriétaires fiables, permissions dangereuses, reparse points et identités. Le contrôle intervient avant `Process.Start`, puis après lancement et aux contrôles renouvelés.

Les fixtures exécutent le conducteur livré, avec interception empêchant tout lancement WSL. Elles vérifient :
- refus avant démarrage pour ancêtre substituable, reparse, identité changée ou observation impossible ;
- refus au renouvellement, à AUTHORIZE et WRITE ;
- absence d'ALLOW périmé ;
- positifs avec lecteurs publics et propriétaires système fiables.

### S-2 — FERMÉ

**Localisation :** `local-private-parent.py:361–417` ; `local-private-parent.ps1:533–536`.

L'état final `RETENTION_UNPROVEN` survit désormais à l'exception du contexte, devient une frame fermée `ABORT / FAIL / RETENTION_UNPROVEN`, puis déclenche explicitement l'avertissement Windows.

Les parcours synthétiques après ACK et à échéance vérifient le non-zéro, le relevé restant, l'absence de CLOSED/publication/contenu natif et la fermeture des descripteurs. L'avertissement est également exercé dans le conducteur Windows livré.

## Reasoning

- **Passe défensive :** contrôles présents et correctement câblés avant activation puis renouvelés ; diagnostic de rétention propagé sans exception libre.
- **Passe adversariale :** feuilles inchangées avec ancêtre hostile refusées ; changement entre phases refusé ; retrait impossible ne redevient plus une simple entrée invalide ni une clôture normale.

## Tests

**Reviewer :**
- Six tests ciblés : **6/6, 11,401 s, exit 0, aucun skip**.
- TEMP/TMP explicitement dirigés vers le répertoire approuvé.
- `git diff --check` : **exit 0**.
- Blobs gelés reconfirmés après exécution.

**Preuve du lead réutilisée :** suite Windows **194 tests, exit 0, skips 8, 186 exercés**. Le premier essai avec TMP absent reste un échec de précondition documenté, pas un ROUGE produit.

## Do not redo

Ne pas rouvrir le consommateur privé, sa grammaire ou ses anciens findings. Ne pas transformer le refus du stockage réel en exception ni modifier ses ACL pour obtenir un résultat vert.

## Anti-goals

Aucun WSL, Chromium/k6, Cargo, CI, installation, préflight réel ou collecte. Aucune modification de code ou de ledger par ce reviewer.

## Open questions

Pas de preuve Linux réelle du correctif ni de validation opérationnelle du stockage. Ces limites restent distinctes de la fermeture des deux défauts.

## Staleness

Cette fermeture porte sur les trois blobs ci-dessus, **pas sur HEAD seul**. Tout changement de ces fichiers exige une vérification du delta.

## Provenance de persistance

Retour natif read-only `result_mvv56i1b_0002_20d7dm`, session
`ses_e72d3280fffelBiYRwaxmXJwey`, tâche `task_mvv24mb0_0033_i8mynp`.
Le lead persiste le contenu complet ; l'index de mission est actualisé plutôt
que l'index général contenant des entrées étrangères. Les changements de HEAD
documentaires depuis la revue ne modifient pas les trois blobs concernés.
