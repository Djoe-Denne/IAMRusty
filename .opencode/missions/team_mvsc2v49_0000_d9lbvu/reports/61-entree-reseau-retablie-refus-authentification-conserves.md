# Entrée réseau rétablie, refus d'authentification conservés

L'opération ciblée réussit dans le laboratoire : la route de connexion répond
avec la redirection attendue et son cookie temporaire. Les trois routes protégées
refusent les requêtes sans authentification, sans cookie et sans appel IAM
supplémentaire pendant ces refus.

Une seule entrée réseau a été ajoutée : l'adresse de tunnel mesurée vers le port
d'Envoy. L'accusé de réception de la modification et la relecture correspondent.
Les24 autres règles, le Service et l'identité d'Envoy sont préservés ; les tunnels
temporaires de contrôle sont fermés. La vérification infra indépendante examine
maintenant ces preuves avant les phases suivantes.

Ce succès concerne seulement l'entrée du laboratoire. Le rejet d'un certificat
de mauvais nom par Envoy et la matrice du vrai navigateur restent à exécuter ;
aucune clôture globale ni qualification de production n'est revendiquée.

## Preuves techniques

- Livraison3c2e7ea, wrappera551… : exit0 en136.671s.
- Résultat PASS_LAB_INGRESS_ONLY ; positif303, nonce et redirect validés en mémoire ;
  trois négatifs403 sans cookie.
- NetworkPolicy UID inchangé, RVACK62929 ;25policies comparées au journal,
  seule l'addition attendue dans allow-envoy diffère.
- Preuves : dossier `envoy-ingress-7be2f63603864e1b9e7b280630a47c9a`,
  journal, ACK, witness, summary et parent-postmetadata.
- Le refus initial avant mutation reste consigné séparément, pas transformé en PASS.
