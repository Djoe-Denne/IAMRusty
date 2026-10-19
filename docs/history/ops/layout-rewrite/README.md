# layout-rewrite

> Archive historique de migration de layout, non nominale pour le runtime.
> Ancien chemin : `ops/scripts/layout-rewrite/`. Les fixtures suivies restent
> des données de self-check, pas des fixtures IT de plateforme.

Réécrit les **citations de chemins** du monorepo AIForAll vers le layout cible. Ne déplace aucun dossier.

Table de vérité : `mapping.json`. Moteur unique : `Rewrite-Layout.ps1` (pas de `.sh` jumeau).

## Dry-run (défaut, aucune écriture)

```text
powershell.exe -NoProfile -File docs/history/ops/layout-rewrite/Rewrite-Layout.ps1
powershell.exe -NoProfile -File docs/history/ops/layout-rewrite/Rewrite-Layout.ps1 -Verbose
powershell.exe -NoProfile -File docs/history/ops/layout-rewrite/Rewrite-Layout.ps1 -ReportPath .tmp-layout-rewrite/report.txt
```

`-RepoRoot` optionnel. Défaut : remontée depuis le script jusqu'au dossier qui contient `Cargo.toml` et `.git`. Ça reste la racine du dépôt depuis l'archive `docs/history/ops/layout-rewrite/`.

## Self-check (fixtures, pas le vrai arbre)

```text
powershell.exe -NoProfile -File docs/history/ops/layout-rewrite/Rewrite-Layout.ps1 -SelfCheck
```

N'exige pas que `services/`, `ops/`, etc. existent dans le depot reel. Code 0 = OK, 1 = echec.

### Limite historique du self-check sur checkout propre

Revalidation statique après archivage (2026-10-19) : **exit 1**, trois assertions
restent en échec : `prefix ./certs/x`, `prefix docker-build-stage`,
`compose volume certs`. Les fixtures suivies citent `certs/x`,
`docker-build-stage/obj` et `certs/platform-mesh`, mais ces données simulées
dépendaient de copies privées ignorées (famille N2), absentes d'un worktree
propre. C'est une limite préexistante de l'archive, pas une preuve de régression
du déplacement ni une suite verte. Les 57 assertions sont conservées à
l'identique ; aucune fixture synthétique n'est ajoutée pour les verdir.
Les copies privées sous l'ancien emplacement ne sont ni copiées, ni archivées,
ni nettoyées. Le parsing PowerShell et la découverte de racine restent valides.

## Apply

```text
powershell.exe -NoProfile -File docs/history/ops/layout-rewrite/Rewrite-Layout.ps1 -Apply
```

Bloqué tant que chaque entrée de `apply_precondition_dirs` n’existe pas. Message français, code 1, **aucun fichier écrit**. Ce script ne fait pas `git mv`.

## Codes de sortie

| Code | Sens |
|------|------|
| 0 | dry-run ou self-check OK |
| 1 | apply refusé (préconditions) ou self-check en échec |
| 2 | usage / racine / mapping.json |

## Refus explicites

- Pas de déplacement / renommage physique (y compris ce toolkit).
- Pas d’écriture en dry-run.
- Pas d’`-Apply` si un dossier destination manque.
- Pas de scan de `rustycog/` (submodule), `target/`, `.git/`, `docker-build-stage/` et `ops/docker/build-stage/` (cache), `.terraform/`, `.vs/`, `docs/history/ops/layout-rewrite/` (sauf fixtures via `-SelfCheck`). Les exclusions des anciens chemins historiques du toolkit sont conservées. Les junctions / reparse points sont ignores.
- Pas de réécriture de `mapping.json` / `Rewrite-Layout.ps1` / README de ce dossier.
- `docs/`, `obsidian/`, `config/` restent à la racine ; leurs **citations** vers d’anciens chemins repo peuvent être réécrites.
- Noms nus en prose (`The Hive service`) : pas de remplacement. Wikiliens vault `projects/hive/...` : inchangés.
- `context: ..` dans un `docker-compose` de service (ex. `IAMRusty/docker-compose.yml`) est réécrit en `../..` une fois le fichier sous `services/` ; `dockerfile: IAMRusty/Dockerfile` devient `services/IAMRusty/Dockerfile`.
- Scan texte (docs, compose, mémoires, etc.) : un chemin n’est réécrit que si la source **ou** la destination mappée existe déjà dans l’arbre courant (ordre réel : `git mv` puis `-Apply`). Idempotent si le token est déjà la destination. Ça laisse passer les images Docker (`openfga/openfga`), les listes (`Hive/IAM/Telegraph/GitHub`) et les features Cargo (`apparatus-contracts/test-harness`) : ni l’ancien chemin ni le mappé n’existent.
- `path =` dans les Cargo.toml et `include_str!` / `include_bytes!` sont réécrits même si la cible n’existe pas encore.
