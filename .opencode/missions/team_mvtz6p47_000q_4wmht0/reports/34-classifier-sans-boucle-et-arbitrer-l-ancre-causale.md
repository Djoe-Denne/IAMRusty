# Classifier sans boucle ; arbitrer en parallèle l'ancre causale

## Problème et décision

Nous savons que le navigateur direct 22 s'arrête par SIGABRT avant cleanup, mais pas pourquoi. L'expert confirme que « non classé » confond absence de texte et texte non reconnu. Une simple présence de stderr ne suffirait pas à identifier une cause.

Le lead retient deux travaux indépendants : une classification très courte des formats déjà capturés, dans le contrat actuel ; et une proposition architecturale sur une observation causale précise si cela ne suffit pas. La seconde branche ne donne aucune permission d'exporter une ligne native ou sa localisation. Son implémentation éventuelle attendrait intégration et ratification explicites.

## Limites et oracles

La classification n'ajoute ni collecte, argument ou protection modifiée. Elle reconnaît seulement format fatal Chromium, assertion rapportée, autre texte ou aucun texte, sans corps, chemin, condition, PID ou suffixe. Le texte inconnu ou une simple famille ne doit pas déclencher une série de reconnaisseurs spéculatifs.

L'architecte doit chercher une ancre minimale utile et sûre, ou préciser le plus petit arbitrage réellement manquant. Pas de corpus massif, nouveau framework, sonde, privilège, stderr brut ou remède AppArmor implicite. Aucun nouvel essai n'est lancé à ce stade ; les129tests Linux et le SIGABRT précédents restent les seules preuves natives courantes.

## Organisation et preuves

Le même auteur poursuit les fichiers k6 ; l'architecte travaille dans un nouveau contrat Proposed séparé. Les chemins ne se recouvrent pas. Le lead conserve intégration, slot Cargo, preuve et publication ; les consommateurs d'ancre restent non admis jusqu'à décision.

- Suivi expert `result_mvu9thwl_0000_1wcyyv`, session conservée, sources Chromium156 logging/debugger et glibc2.35 assertions/abort relues ; aucune cause ou version de bibliothèque effective prétendue prouvée.
- Scope source : complément du briefing `20261030-k6-direct-stop-facets-implementation.md`, base783fa80.
- Scope architecture : `20261030-k6-causal-anchor-architecture.md`, proposition seulement.
- Photographie de l'ordre mission1 actualisée, texte de préparation historique conservé. README sera harmonisé avec les faits au prochain delta vérifié, sans lancement identique pour un edit documentaire seul.
