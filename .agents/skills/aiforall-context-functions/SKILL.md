---
name: aiforall-context-functions
description: "Réutilise les fonctions locales validées Context Mode AIForAll : lectures comparatives, classification de commandes, lectures indépendantes, comparaison d'inventaires et décodage strict UTF-8/UTF-16. À charger pour ces analyses ou pour évaluer une candidate ; ne remplace pas les outils de recherche."
---

# Fonctions d'analyse — AIForAll

Bibliothèque **locale au projet**, sans dépendance npm, serveur supplémentaire ou
cache de résultats. Runtime : Node >= 18 ; chargement CommonJS également vérifié
dans le runtime JS de Context Mode. Les permissions du client et le contrat
`AGENTS.md` restent applicables ; ce module n'est pas un confinement OS.

## Choisir sans multiplier les appels

- Lecture courte connue, symbole ou matching exact : garder les outils dédiés.
- Analyse comparative de plusieurs extraits : `readEvidence`.
- Interprétation fiable d'un résultat de processus : `classifyCommandResult`.
- Lectures indépendantes dans un script : `collectIndependentReads`.
- Inventaires de conteneurs déjà capturés : `compareOwnedContainerInventories`.
- Octets de preuves UTF-8/UTF-16 : `decodeEvidenceText` (module seulement).
- Voir [catalogue.json](catalogue.json) seulement pour les signatures/limites
  utiles. Ne pas relire le code des fonctions validées à chaque invocation.
- Un traitement inhabituel simple peut rester un script libre. Pas de quota,
  consultation cérémonielle ni sauvegarde systématique des snippets.

## Invocation depuis Context Mode

Découvrir la signature réelle de `ctx_execute` dans le catalogue MCP. Envoyer le
code ci-dessous **à cet outil** (pas au Code Mode confiné du client, qui ne fournit
pas require/fs). Remplacer `root` par la racine du checkout effectivement vérifié,
y compris dans un worktree ; ne pas importer le module d'un autre checkout.

```javascript
const root = 'C:/Users/djden/source/repos/AIForAll';
const helpers = require(root + '/.agents/skills/aiforall-context-functions/scripts/helpers.cjs');
const result = helpers.readEvidence({
  root,
  files: [
    { path: 'AGENTS.md', ranges: [{ start: 1, end: 8 }] },
    { path: '.agents/skills/aiforall-mcp-workflow/fallbacks.md', ranges: [{ start: 6, end: 20 }] },
  ],
  maxOutputChars: 10000,
});
console.log(JSON.stringify(result));
```

Le module se charge par chemin absolu : le cwd JS du sandbox est temporaire.
Un root relatif, une traversée hors projet ou un lien sortant sont refusés.
Les fichiers sont préflightés avant lecture ; les entrées en échec ne masquent
pas les autres. Chemins/lignes sont conservés, troncatures et budgets explicites.
Élargir/cibler une lecture coupée avant toute conclusion d'absence.
Les règles d'accès et la confidentialité doivent être vérifiées **avant** de
fournir un fichier ou d'imprimer son contenu ; aucun helper ne les contourne.

Alternative CLI (dans Context Mode pour les sorties longues) :
`node <chemin-absolu-du-skill>/scripts/run.cjs list`, `describe <id>`, ou
`invoke <id>` avec un objet JSON sur stdin. Passage par tableau d'arguments, sans
shell ; ne pas interpoler un gros objet JSON dans une commande PowerShell.
Seules les opérations du catalogue sont dispatchées ; pas d'eval/import dynamique.
Pour construire l'appel imbriqué sans échappement manuel, voir
[examples.md](examples.md) : paramètres sérialisés, code d'invocation court.

## Commandes et erreurs

Le classificateur **n'exécute aucune commande**. Pour un résultat de spawnSync,
utiliser `classifyCommandResult(result, { noMatchExitCodes: [1] })` pour **rg** ;
ne pas appliquer cette convention aux autres exécutables sans vérification.
Pas de stdout, stderr ou error.message recopié par défaut. Un diagnostic déjà
sélectionné et non secret peut être fourni explicitement dans `diagnosticText`.
Ne pas sérialiser l'objet Error brut : il peut contenir tout stdout/stderr.

La collecte indépendante attend toutes les opérations et conserve leurs labels,
valeurs ou erreurs bornées. **fulfilled ne prouve pas le succès métier** : examiner
les réponses (isError, code de sortie, pagination/troncature). Cette fonction n'est
pas destinée aux mutations ou aux opérations dépendantes.

## Sauvegarder et promouvoir

Les deux fonctions promues le 2026-10-28 sont dans `scripts/`, exportées par
`helpers.cjs` et décrites dans le catalogue. Les dossiers de provenance sous
`candidates/` restent des archives, jamais importés à l'exécution.

Pour capitaliser les scripts d'une mission terminée, invoquer explicitement
`aiforall-context-ingest` (repli : `.agents/skills/aiforall-context-ingest/SKILL.md`
depuis la racine du checkout). Opération distincte, jamais automatique à la clôture ;
mode inspect par défaut, prepare pour des candidates si les écritures sont autorisées.

Après un traitement non trivial réussi/corrigé, évaluer : valeur probable de
réutilisation, complexité évitée, paramètres stables, absence d'un outil existant.
En read-only ou hors mandat : proposer dans le retour, ne rien sauvegarder.

Si la tâche autorise ces écritures, créer via les outils d'édition un dossier
`candidates/<slug>/` avec code paramétré, tests et métadonnées selon
[candidate-template.json](candidate-template.json). Utiliser `status: candidate`.
Ne conserver ni secrets, logs bruts, données de la demande ou chemin personnel.
Ne jamais écraser une candidate existante ; inspecter/versionner son évolution.
Avant d'en créer une autre, localiser les métadonnées des candidates pertinentes
avec les outils fichiers, sans importer/exécuter leur code pour les découvrir.

Les candidates ne sont **jamais chargées par le catalogue ou la CLI**. Leur test
exécute du code : lire/revoir d'abord ce code et obtenir les permissions requises.
Pour promouvoir : revue explicite du diff, tests positifs/négatifs pertinents et
invocation dans Context Mode, puis ajout volontaire à l'allowlist du runner et au
catalogue avec preuve dans [VALIDATION.md](VALIDATION.md). Si cela dépasse le mandat,
laisser candidate/proposer la promotion. Aucun auto-enregistrement après un succès.

## Vérification et limites

Test ciblé sans infrastructure :
`node --test --test-reporter=tap <chemin-du-skill>/scripts/helpers.test.cjs <chemin-du-skill>/scripts/compare-inventories.test.cjs <chemin-du-skill>/scripts/decode-evidence.test.cjs`.
Les tests utilisent uniquement leurs propres petits fichiers temporaires.
Pas de Cargo, Docker, Kind, WSL, dépendance ou serveur à démarrer.
Voir [VALIDATION.md](VALIDATION.md) pour les résultats réels et limites.
