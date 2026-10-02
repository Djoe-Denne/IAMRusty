# layout-rewrite

Réécrit les **citations de chemins** du monorepo AIForAll vers le layout cible. Ne déplace aucun dossier.

Table de vérité : `mapping.json`. Moteur unique : `Rewrite-Layout.ps1` (pas de `.sh` jumeau).

## Dry-run (défaut, aucune écriture)

```text
powershell.exe -NoProfile -File scripts/layout-rewrite/Rewrite-Layout.ps1
powershell.exe -NoProfile -File scripts/layout-rewrite/Rewrite-Layout.ps1 -Verbose
powershell.exe -NoProfile -File scripts/layout-rewrite/Rewrite-Layout.ps1 -ReportPath .tmp-layout-rewrite/report.txt
```

`-RepoRoot` optionnel. Défaut : remontée depuis le script jusqu'au dossier qui contient `Cargo.toml` et `.git`. Ça reste la racine du dépôt après le déplacement de `scripts/` vers `ops/scripts`.

## Self-check (fixtures, pas le vrai arbre)

```text
powershell.exe -NoProfile -File scripts/layout-rewrite/Rewrite-Layout.ps1 -SelfCheck
```

N'exige pas que `services/`, `ops/`, etc. existent dans le depot reel. Code 0 = OK, 1 = echec.

## Apply

```text
powershell.exe -NoProfile -File scripts/layout-rewrite/Rewrite-Layout.ps1 -Apply
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
- Pas de scan de `rustycog/` (submodule), `target/`, `.git/`, `docker-build-stage/` (cache), `.terraform/`, `.vs/`, `scripts/layout-rewrite/` (sauf fixtures via `-SelfCheck`). Les junctions / reparse points sont ignores.
- Pas de réécriture de `mapping.json` / `Rewrite-Layout.ps1` / README de ce dossier.
- `docs/`, `obsidian/`, `config/` restent à la racine ; leurs **citations** vers d’anciens chemins repo peuvent être réécrites.
- Noms nus en prose (`The Hive service`) : pas de remplacement. Wikiliens vault `projects/hive/...` : inchangés.
- `context: ..` dans un `docker-compose` de service (ex. `IAMRusty/docker-compose.yml`) est réécrit en `../..` une fois le fichier sous `services/` ; `dockerfile: IAMRusty/Dockerfile` devient `services/IAMRusty/Dockerfile`.
- Scan texte (docs, compose, mémoires, etc.) : un chemin n’est réécrit que s’il existe déjà dans l’arbre courant. Ça laisse passer les images Docker (`openfga/openfga`) et les listes (`Hive/IAM/Telegraph/GitHub`).
- `path =` dans les Cargo.toml et `include_str!` / `include_bytes!` sont réécrits même si la cible n’existe pas encore.
