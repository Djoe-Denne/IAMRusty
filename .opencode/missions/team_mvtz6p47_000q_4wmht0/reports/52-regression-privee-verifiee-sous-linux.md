# Régression privée vérifiée sous Linux

## Résultat

Le correctif du refus privé passe les **165 tests Linux sans skip**, sur une copie
propre du code conservé. Le même code passe 165 tests Windows, avec 8 tests réservés à Linux non exécutés.
Les nouveaux cas utilisent uniquement des données synthétiques connues et vérifient
que la capture ambiguë ne provoque ni écriture, publication ou fuite dans les sorties.

Ces tests renforcent la preuve du correctif ; ils ne ferment pas à eux seuls la
revue de sécurité et ne qualifient pas le navigateur. Aucun Chromium, application
ou site inconnu n'a été utilisé pendant cette campagne.

## Prochaines conditions

Le reviewer termine la relecture limitée du défaut ; l'architecte précise en
parallèle le véritable contrôle parent avant activation. La collecte reste
désactivée jusqu'à ces vérifications et à un créneau réel de lecture locale.
Il n'y a ni nouveau choix d'hôte ni GO général à redemander.

## Preuves et ressources

- Source propre `19b1cfe18c1cedc032b44701dd5f0032bd1ba621`, outillagecbab06e ;
  25fichiers source/workflow/contrats vérifiés avant et après transfert sur ext4.
- Python3 natif UID/EUID1000, unittest discover sous Child et environnement fermé :
  **165/0.949s/exit0/0skip**, diagnostic nul, cleanup réussi,6identités/0restantes.
- Résultat fermé native-unit-result.json dans la campagne k6-20261030-19b1cfe.
- À la libération du bail, AIForAll est déjà Stopped : aucun stop explicite.
  ARTIST-Ubuntu Stopped et Docker Desktop Running sont inchangés ; disque/outils
  conservés, pas de suppression, Docker créé ou arrêt global.
- Aucune publication source ou relance CI pendant la revue ; dernier distant
  vérifié ca0c994, nouveaux livrables committés localement et conservés.
