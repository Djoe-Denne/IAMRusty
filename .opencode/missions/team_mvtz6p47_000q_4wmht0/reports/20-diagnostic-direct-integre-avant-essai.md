# Diagnostic direct intégré — contrôles avant l'expérience comparative

## Objectif et résultat

Le lanceur peut maintenant démarrer explicitement le même Chromium épinglé, sur about:blank, sans le parseur k6. Ce parcours reste diagnostique : aucun exit natif ou témoin ne peut le transformer en qualification navigateur. Les métadonnées hôte sont des lectures précises et fermées, sans changement de politique.

## Vérification acquise

Les cinq fichiers ont été récupérés et comparés strictement au candidat. Le lead a exécuté92tests Windows :85exercés,7tests POSIX ignorés, exit0. Le commit et son vrai hook de formatage sont réussis. Les tests couvrent les sorties forgées, profil/argv fixes, refus des gardes, zéro sans qualification, privacy/cleanup et condition CI préservant l'échec initial.

## Suite

Une revue indépendante ciblée contrôle ce nouveau chemin d'exécution natif et son raccord CI avant publication. L'expérience suivante comparera k6 et Chromium direct dans le même job/UID/pins/racine privée. Elle ne désactivera aucune sandbox et ne publiera aucun flux brut ou artefact d'échec.

Le résultat de cette expérience n'est pas encore connu. Les recherches entre les essais ont motivé cette différence ; ce n'est pas une répétition du lancement antérieur. HTTP reste indépendamment non qualifié.

## Références

- Source locale intégrée e9af3b673319c62577b82f612f868846ae4dfd79, base14d55e9 ; aucun push de ce delta avant revue.
- Addendum Accepted et briefing d'auteur intégrés avant création dans14d55e9.
- Revue : `.cursor/review-briefings/20261030-k6-direct-chromium-security.md`.
- Aucun runtime local, paquet, politique hôte, helper ou privilège créé/modifié par l'auteur ou le lead à ce stade.
