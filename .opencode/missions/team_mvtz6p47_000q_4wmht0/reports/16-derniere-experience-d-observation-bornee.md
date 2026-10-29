# Dernière expérience bornée : distinguer démarrage, scénario et observation

## Ce que l'analyse a établi

Le chemin Chromium est bien lu dans l'environnement du processus par k6 ; le filtrage de __ENV, l'environnement visible au JavaScript, ne le supprime pas. Cette piste est écartée. Le deuxième essai contient un exit k6 non nul : l'absence d'accès aux namespaces ne peut donc pas expliquer seule tout le résultat.

La cause précise reste inconnue. k6 peut perdre certaines lignes fatales Chromium ; le renderer peut aussi rendre certaines lectures /proc inaccessibles. Ces limites sont étayées par les sources officielles, mais aucune n'est déclarée cause réelle sans observation.

## Décision et preuve attendue

Le lead accepte une troisième et dernière expérience, différente des répétitions précédentes : mêmes outils, scénario et gardes, mais un diagnostic fermé indiquant la catégorie du code natif, l'état du résumé courant, l'observation d'un renderer possédé et le statut des lectures de namespaces. Ces états doivent distinguer un arrêt avant scénario d'une exécution partielle ou d'une observation impossible.

Ce n'est pas une nouvelle méthode de preuve sandbox et aucune exigence n'est assouplie. L'essai ne démarrera qu'après intégration et tests du petit delta. Aucun texte natif, chemin, URL, métrique brute, nouveau fichier d'échec, wrapper, paquet, privilège ou changement système. Si la preuve reste absente, pas de quatrième tentative dans ce budget.

## Organisation et limites

L'ancien auteur est arrêté et son résultat est publié/sauvegardé. Un propriétaire isolé reçoit ce sous-lot de diagnostic ; l'ancien worktree sale n'est pas réinitialisé ou repris aveuglément. Le lead conserve publication, baux et exécution CI. L'objectif HTTP reste indépendamment non qualifié, sans nouvelle cible déployée.

## Annexe technique

- Analyse spécialisée `result_mvu4dt4z_0022_vgmkfd`, session `ses_e7638e2b1ffe275ngcCHOU5143`, READONLY, aucune mutation/runtime.
- Sources k6 v2.3.0 : cmd/state/state.go144–170, test_load.go90–99, browser_type.go41–51, browser_options.go84–100 ; Chromium sandbox_debug_handling_linux.cc pour risque d'observabilité.
- Correction de métadonnée : image réelle runner ubuntu-24.04 **20261025.353.1** ; 20261021.618 désigne le provisionneur, pas l'image. Logs existants recoupés, aucun rerun.
- Contrat local de l'expérience : `.cursor/review-briefings/20261029-k6-final-observation-experiment.md` ; base publiée4e82a36.
