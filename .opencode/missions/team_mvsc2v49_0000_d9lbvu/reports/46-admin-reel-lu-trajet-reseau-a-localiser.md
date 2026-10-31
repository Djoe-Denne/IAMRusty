# Lecture réelle réussie ; trajet réseau encore à localiser

L'objectif reste de qualifier l'authentification via Envoy, puis dans un vrai
navigateur. Le lecteur administratif borné fonctionne maintenant sur l'Envoy
réel : il observe un processus actif, sans redémarrage, et ferme son transfert
temporaire après lecture.

Les compteurs IAM observés sont explicitement à zéro, notamment les connexions,
requêtes et échanges TLS. Le compteur des erreurs HTTP5xx est absent et reste
signalé comme tel. Le résultat global demeure donc INCONCLUSIVE : ces mesures
ne démontrent ni un rejet réseau, ni le contrôle d'un mauvais certificat.

Je poursuis la localisation du trajet avant toute correction de règle réseau.
L'architecte prépare la lecture minimale des règles et compteurs existants qui
permettrait d'établir le passage et le rejet effectifs. Aucun outil n'est installé,
aucune règle n'est ouverte pour tester une hypothèse, et aucune navigation OAuth
n'a été exécutée pendant l'accès administratif.

## Preuves et limites

- Version du lecteur livrée :7c95d6e ;42tests simulés réussis et revue sécurité
  indépendante favorables, preuves réutilisées sans nouvelle campagne.
- Lecture réelle unique v2 : exit0, EnvoyLIVE, époque de redémarrage0,
  même identité du Pod ; absence du marqueur après fermeture.
- Résumé privé : envoy-admin-snapshot-c95ceed1888041e1827d6cd8ab80647c.
- Aucun certificat, configuration réseau, secret, image ou donnée modifié.
- Contrôle bon/mauvais certificat et preuve navigateur restent non exécutés.
