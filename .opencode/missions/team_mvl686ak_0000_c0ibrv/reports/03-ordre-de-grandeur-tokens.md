# 3 — Estimation préalable, sans nouvelle génération

## Objectif et état
Donner un ordre de grandeur de consommation pour reconstruire le RPG AIForAll.
Nouveaux appels génératifs suspendus ; disponibilité MCP réelle encore non validée.

## Faits réutilisés / lecture seule
- `grepai status` : 2050 fichiers, 10572 chunks, watcher non actif.
- Dernier log de reconstruction : environ 24000 nœuds RPG, chunks compris.
- `rpg/indexer.go` v0.37.0 : extraction par fichier et symbole, résumé par fichier,
  puis résumés hiérarchiques. Les chunks ne déclenchent pas chacun une génération.
- La petite sonde précédente rapportait 115 tokens de sortie dont 102 de
  raisonnement. Un seul exemple : ce n'est pas une moyenne représentative.

## Estimation, non mesure
Pour AIForAll seul, sans déduire d'éventuels hits de cache : ordre de grandeur
de 15000–30000 appels, puis quelques centaines à quelques milliers de tokens
par appel selon la taille du contexte et le raisonnement xhigh. Enveloppe de
planification grossière : **10–50 millions de tokens entrée + sortie + raisonnement**.
Cette plage n'est ni une borne garantie ni un quota imposé. Une moyenne de
plusieurs milliers de tokens de raisonnement par appel pourrait la dépasser.
Les sept projets ne sont pas additionnés dans cette estimation.

## Interprétation et limites
L'ordre de grandeur prudent est donc la dizaine de millions, pas le milliard.
Pas de conversion honnête en euros ou pourcentage du quota d'abonnement ChatGPT.
Le champ max_tokens envoyé par GrepAI n'est pas traité comme un plafond démontré
du raisonnement par la chaîne LiteLLM/ChatGPT. Aucun rebuild lancé pour mesurer.

## Suite
Poursuite technique sous le contrat existant possible, mais une reconstruction
réelle reste en pause pendant la discussion de consommation. Aucun nouveau
plafond artificiel de requêtes ajouté.
