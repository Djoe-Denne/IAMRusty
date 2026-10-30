# Protection du code si un processus survit

Le but reste de tester les outils Windows puis l'unique cas navigateur autorisé.
L'auteur annonce les correctifs de création de dossier et de vérification des
bibliothèques Chrome, avec des tests ciblés réussis. Leur source complète doit
encore être récupérée et recoupée : ces résultats ne sont pas une acceptation.

Il a également signalé une limite précise : si un processus ne se ferme pas après
l'arrêt gracieux prévu, la fin du lanceur libérerait les protections temporaires
de ses fichiers de code. Rendre un verdict d'échec ne règle pas à lui seul cette
durée de protection. **Le navigateur reste donc gelé.**

L'architecte examine une solution minimale compatible, sans deuxième superviseur,
attente indéfinie, terminaison forcée ou changement de permissions. En parallèle,
le lead récupère et vérifie la livraison pour intégrer les corrections déjà faites.
Les contrôles k6 sans navigateur pourront avancer lorsqu'ils auront leurs propres
préconditions fermées. La preuve d'isolation Chromium reste inconclusive comme
déjà décidé ; aucune expérience WSL ou Linux n'est reprise.

## Preuves et limites

- Retour natif auteur result_mvvdxztz_0002_rnfyb3 : dispositionR-2 annoncée conforme,
  S-1 implémenté et tests attribués, R-1 encore bloquant sur le chemin survivant.
- Delta annoncé : dixfichiers/83262octets/SHA256
  `46cd605132c9836dd37d4788fad663b86cc38e6c8cd1fc5656f27a6f4cf7947e`.
  Retransmission native ou ID de candidate retenue demandés, pas accès à un arbre
  externe refusé ou copie par un autre outil.
- Tests attribués :22ciblés puis10après correctionZIP, pas rerun demandé et pas de
  PASSlead du nouveau contenu. Aucun vrai k6/Chrome/HTTPWindows encore exécuté.
- [Question technique bornée à l'architecte](../../../../.cursor/review-briefings/20261030-k6-windows-bundle-lifetime-inquiry.md).
  Pas de question d'approbation de jalon ni élargissement de mandat.
