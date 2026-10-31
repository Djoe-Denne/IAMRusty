# Adresse traduite différente : le correctif prévu reste bloqué

La capture réelle réussit et apporte un fait discriminant : la connexion vers
Envoy est traduite avec une autre adresse que l'adresse interne du nœud d'entrée
envisagée dans le premier contrat. Ce contrat n'a donc pas été appliqué.

Une seule requête HTTP a été envoyée. Pendant sa fenêtre, un nouveau tuple de
connexion unique apparaît et le compteur de refus lié au Pod augmente, après
une référence passive stable. Envoy reste actif sans nouveau trafic vers IAM.
Le compteur reste partagé : cette corrélation n'est pas présentée comme une
trace certaine de chaque paquet.

Je fais attester le propriétaire de l'adresse traduite par les métadonnées du
cluster, puis réexaminer le contrat minimal par l'architecte. L'hypothèse d'une
adresse de tunnel du nœud doit encore être vérifiée. Aucun changement d'adresse
autorisée, de règle, de certificat ou de cible n'est fait automatiquement.

## Preuves et limites

- Sourceff12007 et fichier exécuté attestés ; observation complète exit0.
- Avant : compteur22paquets/1320octets, stable sur la fenêtre passive.
  Après l'unique requête : hausse9paquets/540octets, pas neuf requêtes HTTP.
- Identités et règles observées stables ; compteurs IAM sans hausse, même
  incarnation Envoy ; transfert Admin fermé.
- Le binding établit la traduction enregistrée, pas une réponse reçue : SYN_SENT/
  UNREPLIED. Zone absente conservée MISSING avec provenance de filtre distincte.
- Adresse traduite10.244.230.64, différente de l'IP interne172.19.0.5.
- Preuve privée : envoy-tuple-observation-94998d80bc7a40fe813ccbad76e4b812.
- Les preuves certificat et navigateur restent ouvertes.
