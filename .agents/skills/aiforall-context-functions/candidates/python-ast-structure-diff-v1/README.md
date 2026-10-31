# Comparaison structurelle Python — candidate v1

Statut : **candidate**, hors catalogue et runner validés. Créée sur demande
explicite après inspection de la session et de ses agents. Aucun code de trace
n'est exécuté : la logique a été réécrite et paramétrée.

## Contrat

`compare_python_ast(before, after, **options)` reçoit deux textes déjà autorisés.
Elle n'accède ni aux fichiers ni à Git et n'importe/exécute aucune de ces sources.
Fonctions, méthodes, classes, async, définitions imbriquées et sous branches sont
inventoriées. Les noms qualifiés dupliqués sont **ambigus**, pas écrasés.

Le résultat compare aussi l'AST complet du module : changer une constante ou un
import peut donner `moduleAstEqual=false` même si aucune définition n'a changé.
Les lignes sont indicatives et les hashes ignorent les attributs de position.
Les différences prioritaires sont retournées avant les éléments inchangés, dans
une borne globale ; les totaux et omissions restent explicites.
Chaque nom ne présente au plus que cinq occurrences par source ; les nombres
réels et les indicateurs de troncature accompagnent ces exemples.

L'égalité d'AST n'est **pas une preuve d'équivalence fonctionnelle** : environnement,
imports, dépendances et effets d'exécution ne sont pas vérifiés. Les commentaires
ordinaires sont ignorés ; les docstrings, décorateurs, annotations, valeurs par
défaut et commentaires de type sont inclus. Utiliser la même version Python pour
une comparaison donnée. Les entrées autorisées restent nécessaires : les plafonds
ne remplacent pas un sandbox OS pour des données hostiles.

## Exécution explicite

Depuis le dépôt, transmettre un objet JSON UTF-8 sur stdin :

```text
python -B .agents/skills/aiforall-context-functions/candidates/python-ast-structure-diff-v1/function.py
```

Exemple d'entrée :

```json
{"before":"def f(): return 1\n","after":"def f(): return 2\n","options":{"max_output_items":10}}
```

Sortie JSON, sans corps des sources. Une erreur utilise un code stable sans
diagnostic du parseur ni texte d'entrée. Les noms de symboles restent des données
potentiellement privées : revoir le résultat avant publication.

Tests synthétiques, après lecture du code et avec autorisation d'exécution :

```text
python -B .agents/skills/aiforall-context-functions/candidates/python-ast-structure-diff-v1/function_test.py
```

La provenance et les vérifications réellement faites figurent dans `candidate.json`.
Vérifications de préparation : syntaxe Python valide, **17 tests réussis** sous
Windows et **3 assertions smoke réussies** dans Context Mode / Python. Aucun
résultat fonctionnel des anciennes sources n'est qualifié par ces tests.
La préparation n'autorise pas une promotion, une modification du catalogue ou une
publication automatique. Aucune référence à une trace privée n'est nécessaire à
l'exécution du script.
