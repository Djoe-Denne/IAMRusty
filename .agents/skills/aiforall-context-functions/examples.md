# Invocation imbriquée sans échappements manuels

## Code Mode → ctx_execute → fonction locale

Ce code est destiné au Code Mode du client, **après découverte de la signature**.
Le root vient du checkout vérifié. Les données sont sérialisées une seule fois ;
les apostrophes, antislashs et sauts de ligne ne deviennent pas du code.

```javascript
const root = 'C:/Users/djden/source/repos/AIForAll';
const modulePath = root + '/.agents/skills/aiforall-context-functions/scripts/helpers.cjs';
const request = {
  root,
  files: [{ path: 'AGENTS.md', ranges: [{ start: 1, end: 8 }] }],
  maxOutputChars: 10000,
};
const code = 'const h = require(' + JSON.stringify(modulePath) + ');'
  + 'console.log(JSON.stringify(h.readEvidence(' + JSON.stringify(request) + ')));';
return await tools['context-mode'].ctx_execute({
  language: 'javascript', code,
  intent: 'comparaison des extraits choisis, chemins et lignes, erreurs partielles',
});
```

Ne pas importer le module local depuis ce Code Mode : son accès externe passe par
les outils. C'est le code envoyé à ctx_execute qui effectue le require.

## Commande existante → classification

Dans le script exécuté par Context Mode, appeler une commande par spawnSync avec
`[arguments]`, `shell: false`, timeout et maxBuffer adaptés. Transmettre son résultat
au classificateur, puis ne renvoyer que les données utiles. Pour rg uniquement :
`helpers.classifyCommandResult(result, { noMatchExitCodes: [1] })`.

Le helper n'exécute aucune commande et ne choisit pas quoi lire. Ne pas imprimer
le résultat brut de spawnSync ou un Error ; sélectionner d'abord une éventuelle
portion non secrète de diagnostic pour diagnosticText. Pour un autre programme,
vérifier ses codes plutôt que copier la convention rg.

## Lectures indépendantes

Dans le sandbox, fournir à collectIndependentReads des objets `{label, run}`
contenant des lectures autorisées ; attendre le tableau complet. Examiner les
valeurs fulfilled (dont isError/ok/code/troncature), puis sélectionner la preuve
utile avant impression. Les fonctions du client MCP ne sont pas automatiquement
disponibles dans le sandbox : orchestration inter-MCP = Code Mode et allSettled.

## Comparer des inventaires déjà capturés

Depuis le script Context Mode qui a importé `helpers.cjs` :

```javascript
const inventory = {
  before: [], after: [], ownedIds: [],
  beforeComplete: true, afterComplete: true,
};
const comparison = helpers.compareOwnedContainerInventories(inventory);
```

Remplacer les tableaux par les captures autorisées, jamais une capture manquante.
Chaque ligne porte `{id, state}`, ID complet 64hex minuscule. La propriété vient
du registre et des baux vérifiés, pas d'un nom de conteneur. Lire `status`,
`counts`, `truncated`, conflits et changements ; aucun booléen isolé n'autorise
une suppression. La CLI accepte `invoke compare-owned-container-inventories`
avec `{inventory, options?}` sur stdin ; plafond CLI 64KiB, distinct des limites
du module. Un processus CLI exit 0 peut rendre `status: inconclusive`.

## Décoder des octets de preuve

```javascript
const decoded = helpers.decodeEvidenceText(Buffer.from('preuve é', 'utf8'));
// Sélectionner/expurger avant de restituer decoded.text.
```

Module seulement : pas de conversion implicite JSON vers Buffer. Auto utilise
la BOM, sinon UTF-8 ; UTF-16 sans BOM exige `encoding: 'utf-16le'` ou `'utf-16be'`.
Séquences invalides, conflit BOM/option, UTF32 et NUL non autorisé sont refusés.
Aucune lecture de fichier ou interprétation des résultats de tests n'est faite.
