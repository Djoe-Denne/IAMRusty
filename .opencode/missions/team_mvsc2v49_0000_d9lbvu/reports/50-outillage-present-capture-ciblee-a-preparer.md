# Outillage présent ; capture ciblée à préparer

Le préflight réel réussit : la version prévue du suivi des connexions et les deux
modules nécessaires sont déjà présents. Le backend iptables est identifié.
Aucune installation ni aucun chargement de module n'a été nécessaire.

Cette vérification n'a lu ni connexion ni règle de refus. Elle lève seulement
le risque de développer un décodeur pour un outil absent. Je poursuis maintenant
la lecture ciblée prévue au contrat : un nouveau tuple de connexion et une seule
chaîne de refus liée au Pod, après tests et revue de cette tranche.

L'adresse traduite et le rejet effectif restent à établir avant toute correction
réseau. Une corrélation ambiguë restera non concluante ; aucun compteur partagé
ne sera présenté comme la trace certaine d'un paquet.

## Preuves

- Run unique : exit0,20.303secondes pour l'enveloppe entière.
- conntrack1.4.7 ; iptables1.8.9, backend nf_tables ; modules déjà résidents.
- Sourcef0a93a1 et fichier importé attestés ; identités vérifiées avant/après.
- Aucun accès OAuth, certificat, règle, image ou donnée modifié.
