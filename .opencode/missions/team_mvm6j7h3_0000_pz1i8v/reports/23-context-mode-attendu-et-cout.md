# 23 — Context Mode : comportement attendu et portée du constat de coût

## Objectif et stade
Expliquer le résultat du rapport22 et préciser la comparaison réellement
possible. Analyse des preuves existantes uniquement ; aucune nouvelle mesure,
modification de règles, réparation ou campagne exécutée.

## Pourquoi Context Mode n'a pas produit l'économie attendue
`ctx_execute` exécute le code fourni dans son sandbox. Il ne décide pas à la
place du modèle quelles lignes sont utiles et ne remplace pas automatiquement
une sortie brute par une synthèse métier. Ce qui est imprimé revient au contexte.

Dans plusieurs événements déjà cités au rapport22, le modèle a utilisé
`readFileSync`, découpé des tranches et imprimé ces tranches presque entièrement.
Le calcul est bien déplacé dans le sandbox, mais la sélection/synthèse ne l'est
pas suffisamment. Il a surtout regroupé des lectures pour économiser des
allers-retours, au prix de gros résultats renvoyés au modèle.

Les appels concernés n'utilisent pas `intent`. Cet argument peut permettre
l'indexation/récupération sélective d'une sortie longue ; il ne constitue pas à
lui seul une synthèse intelligente ni un plafond garanti de sortie. L'économie
principale doit venir du filtrage/agrégation et des extraits choisis par le script.

Le skill demandait déjà de traiter/filtrer et de rendre des conclusions/extraits
utiles. La trace montre une application incomplète, pas une absence de consigne.
Le contrat restait néanmoins qualitatif : pas de critère de retour concret
attaché à chaque appel, donc la conformité à « utiliser Context Mode » pouvait
être prise à tort pour l'optimisation de la sortie. Consignes de prompt, pas
contrôle technique automatique. Ce n'est pas une panne MCP démontrée.

## Comparaison attendue / observée
Attendu : question précise → traitement dans le sandbox → constat ou inventaire
ciblé avec chemins/lignes, et corps exacts seulement nécessaires à la preuve.
Pour des données à réutiliser, indexation puis recherche ciblée, pas restitution
de toutes les tranches suivie d'une synthèse dans le contexte principal.

Observé : plusieurs réponses composites de 13–22 k caractères, avec beaucoup de
source brute. Exemple du rapport22 : les trois consumers imprimés jusqu'à 135
lignes chacun, environ16k caractères retournés. On pouvait commencer par les
trois getters et fonctions de configuration pertinentes, puis élargir pour les
différences métier. Les extraits exacts restent nécessaires quand ils sont la
preuve ; « synthétiser » ne signifie pas supprimer tous les corps de code.

Un budget indicatif de 4–8k caractères utiles par réponse de ce type aurait été
un objectif raisonnable à discuter, avec dépassement justifié. **Ce n'était pas
un seuil préenregistré de la campagne ni une mesure d'un parcours alternatif.**
Aucun nombre d'appels ou temps cible fiable ne peut être inventé après coup.

## Ce que « coûteux » signifie ici — correction de formulation
Faits :77 appels de premier niveau,37 messages assistant,321,01s écoulées,
286469 caractères cumulés de textes de résultats d'outils, catalogues/skills
inclus ; compteurs de session input106233, cache-read2113536, output8457.
Les entrées larges réintroduites tôt sont transportées dans les requêtes suivantes,
et les tours multiples réexposent un contexte croissant. Les compteurs de cache
sont cumulatifs : **pas 2,1M de tokens uniques, ni de VRAM ni de facturation prouvée**.

Les quatre recherches de chemins absents et certains catalogues redondants sont
des surcoûts évitables indépendants de Context Mode. L'erreur d'échappement et
`rg` absent ont ajouté des détours, mais leur part dans la durée totale n'est pas
isolée. Pas d'attribution de tout le coût à un outil.

Il n'y avait ni budget chiffré préalable ni parcours de contrôle comparable.
La formulation « trop coûteux » était donc trop catégorique si comprise comme
dépassement financier ou performance démontrée mauvaise. Conclusion étayée :
**moins économe en contexte que prévu, avec des redondances observables**.
La réponse métier obtenue reste utile ; on ne démontre pas que tous les appels
étaient superflus ou qu'une réduction donnée aurait gardé la même couverture.

## Recommandation, non appliquée
Mieux préciser pour chaque appel le résultat attendu, effectuer la sélection
avant impression et fixer un budget indicatif de retour avec exception justifiée.
Réutiliser le catalogue et localiser les chemins inconnus avant les recherches
groupées. Préserver les extraits nécessaires à la preuve. Pas de nouveau MCP,
changement de modèle ou hook bloquant déduit de cet audit.

Aucune décision demandée pour cette explication. Règles et backend inchangés ;
aucun runtime, Cargo ou Docker utilisé. Rapport22 reste la source des événements
et métriques ; ce rapport précise leur interprétation, sans nouveau benchmark.
