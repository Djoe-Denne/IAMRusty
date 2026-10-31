# Capture ciblée revue avant exécution

La revue sécurité indépendante de la capture et de son enveloppe est favorable,
sans défaut bloquant identifié. Elle conserve explicitement la limite importante :
un compteur partagé et une fenêtre passive calme ne prouvent pas, seuls, la
causalité d'un rejet.

Je livre maintenant cette source vérifiée. L'observation prévue lira uniquement
les connexions filtrées et une chaîne liée au Pod, avec une référence passive,
une seule requête HTTP exacte, puis une comparaison. Les transferts Admin seront
fermés avant cette requête. Tout format, identité ou compteur ambigu arrêtera
l'observation ; aucune règle réseau ne sera modifiée pour essayer.

Une notation documentaire ambiguë du chemin HTTP a été remplacée par le chemin
exact déjà ratifié. Aucune route ni exemption n'a changé.

## Preuves et limites

-51tests simulés réussis ; unité de refus d'une source non qualifiée réussie.
- Revue fresh du code et de l'enveloppe, empreintes vérifiées.
- Outillage réel déjà vérifié ; données NAT/rejet encore non capturées.
- Aucun outil installé, module chargé, image reconstruite, certificat ou droit
  réseau modifié. La preuve navigateur reste distincte et ouverte.
