# Comparaison intégrée ; contrôles locaux vérifiés avant l'essai Linux

## Résultat

La comparaison Ubuntu22.04 est intégrée. Elle utilise un commutateur fermé du parcours navigateur existant, vérifie les outils puis les deux scénarios, et ne peut lancer qu'un cas navigateur. Même un cas complet reste non qualifiant, non-zéro et sans publication de fichier.

Le lead a vérifié les cinq fichiers à l'identique du candidat et constaté que le job Ubuntu24.04 préexistant est inchangé. Les109tests Windows passent :102exercés,7spécifiques POSIX ignorés. Le contrôle du diff et le vrai hook du commit réussissent. Ce résultat ne prouve pas encore le comportement réel sur Linux.

## Suite et limites

Une revue indépendante ciblée vérifie les refus de plateforme et la séparation entre observations et qualification. L'essai Linux sera lancé après cette revue par publication normale, pas par dispatch ou retry manuel. Il ne modifiera ni politique de sécurité, pin, scénario ou service.

Le refus natif sur Ubuntu24.04 reste connu, mais la cause exacte n'est pas encore prouvée. Ubuntu22.04 est uniquement un témoin temporaire ; HTTP reste sans cible conforme autorisée. Aucun PASS global, démarrage runtime local ou qualification parent-cleanup n'est annoncé.

## Annexe technique

- Source intégrée `d47b80508f71cfffbbd5d6caf2146f949997a954`, base `b316668d79a113281477eb799fb5f20fba5506ab`, cinq fichiers/+455−24 dont nouveau test294lignes.
- Candidat `result_mvu77jja_000r_62uc5k`, session `ses_e75f211bfffe498yEpFr2CniSK` ; arbre initial propre b316, aucun commit auteur, fichier de test initialement non suivi récupéré explicitement.
- Lead : commande Python3.13 native109tests/0.476s/exit0/skips7 ; diff-check normal/staged exit0 ; slot Cargo libre avant hook normal fmt réussi.
- Briefing indépendant : `.cursor/review-briefings/20261030-k6-linux2204-comparison-correctness.md`.
