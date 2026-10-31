# Signataire distant intégré, raccordement IAM libéré

## Objectif et résultat

La mission corrige les frontières de sécurité avant sa qualification complète. Le durcissement du signataire distant est désormais intégré avec ses tests ciblés. Les fichiers IAM sont libérés : leur raccordement à la nouvelle politique de principal peut avancer en parallèle du correctif applicatif et du laboratoire.

## Vérifications acquises

- **Signataire distant :** 29 tests de configuration, 19 du protocole et deux du véritable client TLS réussis, aucun cas ignoré. Bon nom de certificat accepté ; mauvais nom signé par la même autorité et autorité inconnue refusés. Les redirections n'atteignent pas leur cible. Traces natives recoupées ; sept fichiers intégrés identiques au commit testé.
- **Compatibilité :** la liste d'algorithmes absente conserve le défaut effectif RS256. Le témoin utilise le vrai helper, sans lire la clé privée volontairement indisponible. HS256 et les mélanges restent refusés en mode sécurisé.
- **Qualité ciblée :** formatage des trois packages IAM et Clippy ciblé réussis. Le formatage global Windows reste en échec d'environnement pour chemin trop long ; aucun succès global annoncé.
- **Application :** premiers checks de sentinel-sync et des bibliothèques Hive réussis ; six tests purs de projections réussis. Une erreur de namespace a été corrigée par un réexport existant, sans dépendance ajoutée. L'application reste incomplète : ces résultats ne prouvent pas encore la collision intégrée avec deux jetons et OpenFGA réel.

## Suite et limites

Le raccordement IAM doit réellement consommer les deux champs de politique explicitement configurés, sans déduire une plateforme d'un secret, d'un nom d'hôte ou du seul algorithme JWT. Ce sous-lot prévu comme intégration du lead est attribué en isolation après le signataire ; pas de changement des signatures, du stockage ou des règles OAuth.

L'autorité finale du profil navigateur reste gelée pendant la décision complémentaire architecte. Changer l'URL publique modifie actuellement l'issuer effectif d'IAM : les configurations ne seront pas figées sur l'hypothèse contraire. Les tranches indépendantes du laboratoire continuent.

Revue de sécurité globale, tests d'intégration, véritable Envoy et navigateur restent à qualifier ensemble. Aucun Docker, Kind ou WSL de mission démarré. Aucun nouveau GO utilisateur demandé.

## Annexe technique

- Livraison remote `17c9cfd75ed59aee62d7894db5462a036259dae7` ; intégration `6b6d364c5261a3fb72463cd676c7555385388ce0`. Sept fichiers contrôlés, trois références dev-deps seulement dans le lock, aucune version/checksum modifiée.
- Traces `msg_1840185a9001RsYKxlGlqv6bvE` et `msg_1840f5d81001icyscFw5q1qxjU`. Les doublons de présentation des logs ne sont pas des runs supplémentaires.
- Premier merge refusé à cause d'un contrat modifié par le lead : ce seul document a été committé, puis merge repris ; pas de stash/reset. Nettoyage worktree pending, branche préservée.
- Attribution IAM `3192f46230a8f929efe665af6b8e48fc57528e92`, tâche `task_mvsjhanp_000d_ipeun7`, dépendances remote et revue SDK acceptées.
- Slot Cargo restitué après micro-batch final. Tout prochain run exige une nouvelle attribution exclusive.
