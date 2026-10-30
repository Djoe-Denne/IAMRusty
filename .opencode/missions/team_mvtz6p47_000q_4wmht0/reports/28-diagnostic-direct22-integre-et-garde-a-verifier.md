# Diagnostic direct22 intégré ; garde privée à vérifier

## Résultat

Le diagnostic direct22 est intégré comme étape séparée après un refus k6 courant et admissible. Il vérifie le résultat privé, les outils, le contexte et la fraîcheur avant tout nouveau processus, puis interdit une seconde tentative. Il reste non qualifiant, même si Chromium termine normalement.

Le code conserve aussi une catégorie précise du code k6, sans appliquer cette table à Chromium. Les arguments, binaires, scénario et protections ne changent pas. Les sept fichiers ont été récupérés à l'identique du candidat ; les118tests Windows passent, avec7tests POSIX non exécutés. Le vrai hook et le contrôle du diff réussissent.

## Suite et limites

Une revue sécurité indépendante ciblée examine ce nouveau consommateur du résultat privé et sa protection contre les entrées modifiées ou périmées avant l'essai Linux. Elle ne réaudite pas toute la campagne. Aucun lancement natif supplémentaire n'a eu lieu et la cause exacte des refus reste inconnue.

Le fichier privé n'est ni une preuve de propriété runtime, ni un artefact publié, ni une autorisation de modifier la sécurité. HTTP/Chromium et cleanup parent restent non qualifiés. Le job24.04 est inchangé ; la voie AppArmor n'est toujours pas autorisée.

## Annexe technique

- Source `0601f6f73a93a2fedb9b8717450d06acd6a5fa67`, base d47b805 ; sept fichiers/+383−10, nouveau test217lignes.
- Candidat `result_mvu82q4i_000y_dimugy`, même session auteur `ses_e75f211bfffe498yEpFr2CniSK`. ROUGE7/1fail3errors et GREEN ciblé9 attribués à l'auteur ; GREEN final118/exit0/skips7 réexécuté par le lead.
- Aucun delta lock/scénario/provisionnement/safety ni préfixe job24, vérifié par comparaison exacte. Source gelée avant revue ; aucune mutation de branche auteur ou suppression des fichiers non suivis.
- Briefing : `.cursor/review-briefings/20261030-k6-linux2204-direct-followup-security.md`.
