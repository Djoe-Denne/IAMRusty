# 26 — Ajustements agents appliqués

## Résultat et périmètre
Application terminée sur demande utilisateur, en direct sans nouvel agent.
Suite de la [préparation25](25-ajustements-agents-preparation.md), fondée sur le
[diagnostic Serena24](24-serena-latence-diagnostic.md). Aucun code de MCP,
paramètre global, modèle, index/graphe ou cache n'a été changé par ce lot.

## Instructions modifiées
- `AGENTS.md` : obligation compacte de préserver les résultats indépendants,
  attendre les appels lancés, réutiliser l'information valide et cibler avant
  d'élargir. Aucun quota d'appels ni double parcours HIR/Serena imposé.
- `.agents/skills/aiforall-mcp-workflow/SKILL.md` : méthode détaillée et exemple
  `collectIndependentReads` avec `Promise.allSettled`. L'exemple conserve les
  résultats/erreurs par label ; l'agent doit encore examiner les erreurs contenues
  dans des valeurs techniquement résolues avant de conclure à une réussite.
- `.agents/skills/aiforall-mcp-workflow/fallbacks.md` : requête serveur encore en
  cours malgré l'échec d'un appel composite, erreurs d'exécutable non assimilées
  à une recherche vide et séparation file/exécution avant diagnostic de cache.

Les rappels des profils restent inchangés : ils renvoient déjà à ce canon/skill.
Pas de nouvelle duplication dans les34 profils raccordés ni de changement de
leurs permissions, modèles ou rôles. Le keepalive8B reste obligatoire.

## Arbitrage information / temps / contexte
Le texte autorise explicitement une comparaison groupée de fichiers lorsqu'elle
est plus adaptée. Il ne rend pas obligatoires une succession d'appels Serena ou
un résumé qui perdrait une condition décisive. Corps ciblés et sélection dans
Context Mode sont préférés selon le besoin de preuve, pas pour atteindre un
plafond arbitraire. `intent` n'est pas présenté comme un résumeur automatique.

L'attente d'une tâche en cours et le préchauffage ne sont pas présentés comme
une suppression du coût de calcul. Aucun gain de vitesse ou financier n'est
annoncé sans comparaison à couverture équivalente. Pas de préchauffage massif,
nouveau LLM résumeur ou protocole de benchmark ajouté implicitement.

## Vérifications observées
Exécution dans Context Mode du **code exact extrait du bloc JavaScript du skill**,
avec fonctions synthétiques en mémoire, sans appel à un MCP métier ni API :
1. une lecture encore en attente n'est pas déclarée terminée après échec voisin ;
2. son résultat est attendu et conservé ;
3. chaque opération reçoit une sortie étiquetée ;
4. un succès reste disponible malgré le rejet voisin ;
5. une exception synchrone est isolée et étiquetée ;
6. une réponse résolue `isError:true` reste visible pour inspection explicite ;
7. le cas entièrement réussi conserve ses deux résultats ;
8. un lot vide ne produit pas de résultat inventé.

**8 assertions PASS**. Cela valide l'exemple d'orchestration, pas l'obéissance
future d'un modèle à toutes les consignes ou un mécanisme d'annulation réseau.

Métadonnées YAML du skill valides, identifiant inchangé,149lignes (sous500),
référence `fallbacks.md` existante. `git diff --check` ciblé : exit0. Le prompt
de campagne conserve son SHA256 antérieur
`a06aeb3a44daf4d4a61cb549e0dfa9f9069b86e4e7a964dc8ef457b14c2d9e83`.

## Limites et clôture
Modification documentaire/configuration d'agents seulement ; pas de nécessité
d'exploration de symboles applicatifs. Campagne utilisateur non rejouée ; gains
réels et application dans une nouvelle session non mesurés dans ce lot.
Pas de Cargo, Docker, Kind, WSL, appel métier Rust MCP/Serena, redémarrage,
nettoyage ou création de runtime. Les seuls calculs de test utilisent le sandbox
Context Mode existant. Aucun fichier applicatif ni changement utilisateur annulé.

Aucun arbitrage bloquant ; pas de benchmark complémentaire ou de changement
structurel MCP autorisé implicitement par cette livraison.
