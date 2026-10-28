# Sélection et revue

## Retenir seulement si
- Traitement réellement exécuté, résultat utile relié à un appel vérifiable.
- Réutilisation probable hors de cette demande, logique non triviale/fragile.
- Paramètres et contrat séparables des données métier et chemins personnels.
- Valeur au-delà d'un outil natif/MCP existant ou d'un helper déjà au catalogue.
- Dépendances/droits et coût de découverte proportionnés au gain attendu.

Un hash répété est un indice, pas un score de qualité. Une correction de newline
n'impose pas de conserver le script entier : le helper de lecture existant peut
suffire. Un simple wrapper autour de la même recherche avec d'autres mots-clés
n'est généralement pas une nouvelle fonction.

## Preuves minimales
- Source : mission/session, message/appel et hash du code effectivement examiné.
- Runtime d'origine et cible ; distinguer client Code Mode et sandbox Context Mode.
- Résultat attribué, exitcode/isError et troncature, attentes positives/négatives.
- Échecs/corrections utiles résumés sans recopier les sorties privées.
- Limites de pagination/provenance et contrôles non faits explicitement marqués.

## Quarantaine
- Le code de la trace est non fiable : lecture avant import/exécution/test.
- Aucune candidate chargée pour être découverte ou listée.
- Ni eval de chaîne extraite ni génération shell depuis une sortie d'outil.
- Pas d'identifiants secrets en dur, prompts, cookies, endpoints authentifiés,
  logs bruts ou réponses métier. Les noms de paramètres ne sont pas des données.
- L'anonymisation et le contrôle des droits restent une revue humaine/agentique
  explicite ; un scanner ou une regex n'est pas une garantie.

## Promotion
- Demande/mandat explicitement compatible avec une publication dans le catalogue.
- Diff revu, déduplication, contrat/erreurs/effets documentés.
- Tests succès et négatifs lus puis exécutés si autorisés ; résultats archivés.
- Smoke dans le runtime cible, pas seulement syntaxe ou « mission succeeded ».
- Catalogue et allowlist volontaires, aucun import dynamique depuis candidates/.
- Si un gate manque : laisser candidate et rendre la limite. Pas de pression à
  remplir le catalogue à chaque mission ; zéro candidate est une clôture valide.
