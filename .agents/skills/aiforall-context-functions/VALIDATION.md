# Validation de la bibliothèque

## Initialisation validée le 2026-10-24

- Windows natif, Node v24.11.1, aucune dépendance externe.
- Syntaxe des trois fichiers .cjs : node --check, exit0.
- `node --test --test-reporter=tap scripts/helpers.test.cjs` depuis ce skill :
  **24 tests, 24 PASS, 0 FAIL, 0 SKIP**, durée observée 816,2788ms (pas un benchmark).
- Revue directe du code/diff par l'implémenteur : allowlist fixe, aucune mutation
  dans helpers/runner, erreurs sans stdout/stderr brut, préflight lexical/réel,
  fermeture des descripteurs, budgets de contenu/lecture et succès partiels.
- Régression ajoutée pendant la revue : une lecture binaire/UTF-8 en échec compte
  aussi dans le budget de lecture total ; le test est inclus dans les24.

## Couverture

Chemins absents, espaces/Unicode, racine relative/absente, traversée, chemins
absolus/drive-relative, junction sortante (test réellement exécuté), mauvais type,
UTF-8 invalide/binaire, CRLF/LF/CR/fichier vide, plages invalides/hors bornes,
budgets/troncature sans ligne partielle, codes0/1/2 et absence de convention rg
pour les autres programmes. Exécutable absent, timeout, maxBuffer et signal
simulés par objets de résultat, diagnostics bornés sans fuite de sortie brute,
throw synchrone/rejet/succès/erreur métier fulfilled et attente des voisins.

CLI : découverte, description, JSON stdin, erreurs structurées, taille d'entrée,
opération module-only. Catalogue injecté dans une copie temporaire : candidat
invisible/non exécuté ; même une fausse entrée « validated » ne peut ajouter une
opération à l'allowlist du runner. Les fixtures temporaires possédées sont retirées
par le teardown des tests ; aucun fichier de projet n'est supprimé.

## Smoke Context Mode

Chargement réel par require absolu dans **Bun1.4.2** ; cwd différent du checkout,
lecture d'AGENTS.md avec lignes, entrée manquante conservée comme erreur ENOENT
sans masquer la lecture réussie. Collecte de lectures : succès, rejet et réponse
fulfilled isError:true distincts ; message d'exception privé non renvoyé.
Exécution réelle de rg sur stdin : codes0/1 classés success/no-match.
Le skill est découvert et a été chargé par le client, pas seulement posé sur disque.
Smoke après la correction du budget : module/lecture partielle et CLI JSON stdin
fonctionnent. Exemple imbriqué réellement exécuté avec JSON.stringify et un chemin
contenant apostrophes, guillemets et saut de ligne : pas d'erreur de syntaxe ;
ENOENT conservé pour cette entrée, lecture voisine réussie.

## Limites et procédure future

Revue par l'auteur, pas review indépendante. Pas de nouvelle session LLM pour
mesurer le choix spontané de la bibliothèque ; aucun gain financier/latence garanti.
Node18 minimum déclaré par l'API utilisée, mais seuls Node24/Bun1.4.2 ont été
exécutés ici. Pas de qualification Linux/macOS ni d'exécution dans Cursor/Codex.
Les permissions du client restent obligatoires ; les chemins sont vérifiés au
moment du préflight, sans garantir l'absence d'une substitution concurrente hostile
du filesystem. Ce module n'est pas une sandbox ou un système de redaction universel.

Le plafond maxOutputChars couvre le contenu annoté, pas les métadonnées JSON.
Les plages dont la fin dépasse EOF sont bornées avec returnedRanges explicite ;
une plage commençant après EOF est une erreur. Aucun cache de résultats de source.
Les candidates restent exclues de l'exécution. Une future promotion doit ajouter
ses propres preuves de revue/tests/smoke, pas réutiliser celles de cette version.

## Promotion explicitement autorisée le 2026-10-28

Demande utilisateur : « Est-ce que tu peux les inclure dans la bibliothèque ? ».
Les deux candidates issues de context-ingest144 sont désormais des modules fixes
sous `scripts/`, exportés par `helpers.cjs` : `compareOwnedContainerInventories`
et `decodeEvidenceText`. Aucun chargement depuis `candidates/` ou chemin du
catalogue. Provenance conservée dans les dossiers historiques des candidates.

### Revue directe du diff de promotion

Revue par l'auteur, sans revue indépendante. Implémentations déplacées sans
changement de logique ; tests déplacés pour exercer les exports publics. Examen
des consommateurs helpers/runner/catalogue/tests : égalité exports/catalogue,
allowlist fixe et test de quarantaine adaptés aux deux imports locaux fixes.
Ajout de deux tests d'intégration : description/découverte, CLI inventaire,
inconclusive conservé, IDs invalides expurgés, décodeur module-only.

La comparaison est autorisée explicitement dans le runner sous
`compare-owned-container-inventories`, enveloppe `{inventory, options?}`.
Le décodeur reste module-only pour conserver Buffer/Uint8Array sans protocole
JSON binaire supplémentaire. L'exit CLI 0 n'est pas un verdict métier : lire le
statut inconclusive, les comptes et les conflits de propriété.

### Vérifications nouvelles réellement exécutées

- Windows natif, Node **v24.11.1** : sept `node --check` réussis sur les modules,
  le runner et les trois suites.
- `node --test --test-reporter=tap scripts/helpers.test.cjs scripts/compare-inventories.test.cjs scripts/decode-evidence.test.cjs` :
  **49 tests, 49 PASS, 0 FAIL, 0 SKIP, 0 CANCELLED**, exit 0, stderr vide.
- Context Mode **Bun 1.4.2** : import du module public, dix assertions PASS sur
  inventaire actif/incomplet/ID invalide, UTF16 et UTF8 invalide, liste des cinq
  fonctions, CLI/module identiques et refus d'invoquer le décodeur via CLI.

Les limites des candidates restent applicables : snapshots non atomiques et
complétude déclarative, comparaison ID/état sans volumes/exit codes/baux, aucune
preuve autonome de propriété ni autorisation de suppression. Décodeur : UTF-8
par défaut sans BOM, aucune devinette Latin1/UTF32, texte non expurgé ; entrée
déjà allouée et pas de snapshot garanti d'un buffer partagé concurrent.
Pas de qualification Node18/Linux/macOS ajoutée. Aucun Docker/Cargo/Kind/WSL,
dépendance ou serveur démarré. Aucun commit/push dans ce mandat.
