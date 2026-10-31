# IAM intégré et configurations navigateur débloquées

La mission vise à corriger puis qualifier l'authentification sans changer inutilement l'architecture. Le raccordement IAM est maintenant intégré avec le commit exact du SDK publié. La configuration navigateur peut être finalisée : son port d'entrée reste18080, tandis que l'autorité signant les jetons reste8080.

## Changements vérifiés

- IAM consomme explicitement la politique du principal et refuse son absence ou une autorité incohérente avant appel au signataire. L'oracle a échoué avant correction, puis réussi ; les suites auteur de configuration (30 tests), setup (14) et seed (5) sont vertes. Les neuf fichiers intégrés ont été comparés au commit auteur. Le code combiné complet n'est pas encore qualifié.
- Le SDK publié est sélectionné dans le dépôt principal. La configuration Serena étrangère a été conservée, avec empreinte identique avant/après ; elle n'est pas embarquée dans le commit.
- L'architecte a confirmé une solution plus petite que le changement d'autorité envisagé : garder public_base_url et l'autorité8080, enregistrer séparément les deux callbacks navigateur18080 dans IAM et GitLabConnect. Le code existant distingue déjà ces deux rôles. Le contrat est ratifié, les anciennes exigences contradictoires amendées et les valeurs transmises à l'auteur du laboratoire. Aucune migration d'identité ni nouvelle API n'est nécessaire pour ce choix.
- Les tests Hive compilent désormais et sept tests purs des règles d'autorité passent. Cela ne prouve pas encore l'isolation des droits avec base et OpenFGA réels.

## Limites et suite

La suite SDK RS256 activée par la fonctionnalité testing a révélé un import de test manquant avant toute exécution. Ce n'est pas une régression cryptographique démontrée. Une correction mécanique bornée est attribuée, avant la baseline puis l'adaptation des fixtures existantes ; aucun changement de production ou d'assertion n'est autorisé pour obtenir le succès.

Un chemin d'auto-invitation interne Hive utilise encore le seul identifiant utilisateur. La même frontière de sécurité déjà ratifiée doit aussi y être appliquée lorsque l'auto-invitation est activée, sans interdire les synchronisations ordinaires. Le correctif et ses témoins sont attribués au même auteur.

Le laboratoire est en finition ; aucun Docker, Kind, WSL ou véritable navigateur n'a été démarré. Les IT finales, la revue indépendante du diff intégré et la preuve réseau réelle restent nécessaires. Le formatage global demeure en échec d'environnement Windows, pas en succès implicite. Aucun nouveau GO utilisateur n'est demandé.

## Traçabilité technique

- IAM intégré : `c98b6ea37a9895e67dccaf6d58a14c075a10aeb4`, neuf fichiers auteur `b2f139b` et gitlink SDK `72845d9` ; tâche acceptée après intégration.
- Contrats ratifiés : `21ea615285c70a2f71ae68e30e1ccf4d1a9ac24f`, décision architecte v2 `729d653` relue intégralement. Merge natif refusé à cause du sous-module modifié par l'utilisateur : aucune suppression/stash/force ; décision transcrite par patch documentaire, branche originale conservée. Pas d'identité intégrale de contenu revendiquée.
- Inventaire hors ligne vérifié : neuf overlays historiques, dont Knative déjà présent, puis dix avec le profil navigateur. Égalité fermée conservée ; aucun rendu ignoré pour rendre le gate vert.
- Slot Cargo actuel : exclusivement l'auteur identité pour la baseline SDK et ses fixtures. Doublon IAM détecté avant toute mutation, invité à finir sans nouvelle implémentation.
