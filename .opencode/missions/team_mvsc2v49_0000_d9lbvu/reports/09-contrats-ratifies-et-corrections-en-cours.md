# Authentification : contrats ratifiés et premières preuves ciblées

## Objectif et état actuel

La mission corrige les frontières de sécurité de l'authentification avant sa qualification complète. La séparation des autorités et le transport du signataire distant ont désormais un contrat unique, ratifié et intégré. Le correctif d'identité est en cours ; le correctif distant peut commencer sur cette base. Aucun nouvel accord utilisateur n'est nécessaire.

## Changements vérifiés

- **Permissions :** le plan de correction est approuvé, avec une copie isolée du SDK. L'auteur a demandé le premier créneau de compilation ; il lui est réservé exclusivement pour les contrôles SDK natifs, sans infrastructure de test. Aucun résultat de compilation n'est encore acquis.
- **Signataire distant :** HTTPS obligatoire en mode sécurisé, aucune redirection ni proxy implicite, réponses plafonnées et délai total de dix secondes incluant l'obtention du credential. Une signature conserve son plafond de 4 096 octets ; la clé publique, 16 384. Aucun repli vers une autre clé. Le contrat est intégré ; ces exigences restent non implémentées à ce jalon.
- **Passerelle Envoy :** les deux configurations et leur test ont été examinés. Les traces natives montrent quatre tests statiques échouant avant correction, puis quatre réussis après correction, sans erreur d'environnement ni cas ignoré. La livraison Git puis l'intégration sont en cours. Ces tests ne prouvent pas encore le refus effectif d'un mauvais certificat par Envoy.
- **Navigateur :** les deux documents d'alignement ont été relus. La proposition conserve l'entrée HTTP locale prévue, les échanges internes chiffrés, un vrai connecteur GitLab et un fournisseur simulé avec PKCE, la protection liant le code OAuth au navigateur. Livraison documentaire autorisée ; ratification et intégration restent à terminer.

## Arbitrages et limites

Deux auteurs avaient préparé le contrat distant. Le lead a comparé les textes et retenu une seule version, avec leurs précisions convergentes. La proposition alternative reste préservée et non ratifiée ; aucune seconde implémentation n'est ouverte. Une nouvelle ADR n'est pas nécessaire pour préciser les garanties du fournisseur déjà accepté.

Les hooks Git des seuls livrables documentaires ou non Rust peuvent être omis pour une invocation précise après contrôle du diff. La configuration persistante reste inchangée ; cela n'est ni un succès de formatage Rust ni une dispense de vérification finale.

Aucun Docker, Kind ou WSL n'a été créé ou redémarré pour la mission. Les tests d'intégration et la preuve avec un véritable navigateur et Envoy restent à exécuter après intégration des corrections. Aucun succès global de sécurité, de production ou de fournisseur matériel n'est annoncé.

## Suite autonome

Intégrer les livraisons Envoy et navigateur, libérer leur consommateur commun, poursuivre les deux corrections Rust en parallèle sur leurs fichiers distincts et sérialiser leurs commandes Cargo. Les revues indépendantes et la campagne réelle restent soumises aux prérequis du plan approuvé, sans nouvelle demande de GO de principe.

## Annexe technique

- Contrat identité et attributions : `6a3268b` puis `1ad1e9f` ; ADR0314 Accepted / Unimplemented.
- Contrat remote ratifié : `ded363f03710c6b692290e499b979b0b4d3adfda`, trois documents contrôlés, aucun push applicatif.
- Producteur remote accepté : `task_mvsf2g5q_0004_ft9ns7` ; consommateur publié : `task_mvshoslm_0019_6lzhqb`.
- Envoy : commande `python -B -m unittest discover -s ops/tests -p test_envoy_upstream_server_identity.py -v` ; ROUGE : quatre tests, quinze variantes en échec ; VERT : quatre tests, zéro échec. Traces natives `msg_183c57423001zbLGlitbUJxY2u` et `msg_183c5b254001HGoq47xukwG3yB` recoupées avec le diff exact des trois fichiers.
- Un appel de lecture de preuve a rencontré une erreur de syntaxe du script ; corrigé dans Context Mode. Ce n'était ni une panne MCP ni un échec fonctionnel.
- Nettoyage de deux worktrees après merge resté pending ; branches préservées, aucune suppression forcée ou répétition aveugle du merge.
