# 7. Scénarios relus, raccord Windows à compléter

## Objectif et étape

Les deux scénarios sont présents dans l'arbre de leur auteur et ont été lus intégralement : parcours navigateur complet et témoins HTTP du certificat. Ils ne sont pas encore intégrés ni exécutés. Le raccord Windows reste à compléter avant de créer le laboratoire.

## Ce qui est vérifié

Le scénario navigateur force l'inscription202 puis sa complétion, une nouvelle connexion200 et une réassociation réelle ; il exige les trois refus403, observe cookies et headers du même contexte et projette26checks fermés. Le témoin certificat utilise un seul GET sans suivre de redirection par phase, avec303/503/303 et deux checks ; la preuve SAN et la restauration demeurent au parent. La bibliothèque de résumé partagée n'est pas modifiée.

Le candidat parent contient pour l'instant les validations de résultats/environnement, pas encore la branche de lancement authentification dans l'entrée Windows. Ce livrable est donc partiel et la tâche n'est pas déclarée terminée. Le même auteur poursuit le raccord avec les mécanismes existants ; aucun nouveau backend ni framework demandé.

## Décision de coordination

Le bail réel et les identités seront contrôlés au lancement, après création autorisée. Leur absence actuelle ne bloque pas l'écriture du code qui effectuera ces contrôles. Cela évite la boucle impossible « code terminé avant laboratoire, mais laboratoire exigé pour finir le code ». Aucun contrôle de cible n'est supprimé : seuls son moment d'exécution et la responsabilité du parent sont clarifiés.

## Preuves et limites

Aucun k6/Chrome, HTTP, build, création de nœud ou Secret lu. Les APIs k6 utilisées et les validations statiques restent à rapprocher du rapport consolidé de l'auteur ; le simple fichier présent n'est pas un PASS. Les premiers retours natifs étaient des accusés de réception : réponses techniques complètes demandées, pas de livrable accepté sur cette seule base.

La correction minimale de conservation des nœuds en cas d'échec, son assertion ciblée et la précision --patch-file sont committées dans a43b0be avec hook fmt réussi. La création reste derrière intégration, revue bornée et contrôles requis/réutilisés ; pas de nouveau GO de jalon demandé.
