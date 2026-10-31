# Réseau prêt, préparation du déploiement OAuth

La mission vise toujours la qualification des frontières d'authentification.
L'installation réseau a réussi sur les trois nœuds existants : ils sont tous
prêts, avec les mêmes identités. Aucun conteneur préexistant n'a changé d'état.
Calico et le service DNS interne ont terminé leur déploiement.

La revue sécurité du dernier correctif ne relève aucun défaut dans ce périmètre.
Je poursuis avec les dix images déjà vérifiées, puis le profil OAuth autorisé.
Un contrôle indépendant du réseau est demandé en parallèle. Aucun nouveau
cluster, suppression ou redémarrage de nœud n'est prévu.

Ce résultat prouve la disponibilité du laboratoire, pas encore le comportement
d'Envoy, des permissions ou du navigateur. Ces preuves restent ouvertes.

## Annexe technique

- Reprise réseau : exit 0, source opérations 6704176, SDK02e5.
- Preuves : session privée6b7…/cni-resume-a26b88dee1104886a8f23498d22e0d9c.
- Trois UID Node inchangés ; inventaire Engine avant/après identique.
- Calico, ses contrôleurs et CoreDNS : rollout réussi ; trois conditions Ready.
- Images restent compilées au cf02c4a, sans nouvelle étiquette de provenance.
