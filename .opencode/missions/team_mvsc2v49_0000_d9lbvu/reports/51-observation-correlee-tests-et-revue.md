# Observation corrélée : tests réussis, revue avant capture réelle

La lecture ciblée est implémentée : elle compare un état préalable privé et un
état après une seule connexion, sans remettre les anciennes connexions au caller.
Les identités, règles et compteurs doivent rester cohérents ; un résultat ambigu
ne donne pas autorité pour modifier la règle réseau.

Les51tests simulés passent. Un défaut de partage d'objet a été détecté avant usage
réel : modifier la projection retournée pouvait altérer la référence privée de
comparaison. Une régression rouge puis verte vérifie maintenant son isolation.

La revue sécurité indépendante, avec contexte neuf, examine cette tranche avant
livraison et capture. Aucun listing de connexions, lecture de règle ou requête
d'authentification n'a encore été lancé sous cette nouvelle capacité.

Le format de zone a aussi été clarifié par les sources : un champ absent reste
explicitement inconnu ; la provenance du filtre demandé est indiquée séparément.
Il n'est jamais remplacé par un zéro prétendument observé.

## Limites

- Les tests utilisent des données synthétiques et ne prouvent pas le trajet réel.
- Un compteur de refus partagé n'identifie pas, seul, le paquet concerné.
- Aucun outil installé, module chargé, règle, certificat, image ou donnée modifié.
- Preuves certificat et navigateur toujours ouvertes.
