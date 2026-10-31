# Refus préventif : règles du profil navigateur non reconnues

L'objectif reste de qualifier l'authentification par Envoy puis par un vrai
navigateur. La première exécution réelle s'est arrêtée avant toute modification
réseau : le vérificateur ne reconnaît pas les trois règles déjà déclarées par le
profil navigateur autorisé. Ce n'est pas un échec du trajet HTTP : aucune requête
du scénario n'a été envoyée, aucun journal de mutation ni accusé de réception créé.

La lecture ciblée confirme que le laboratoire contient ces règles du profil
existant. Je corrige uniquement leur reconnaissance depuis leur déclaration
versionnée, sans ignorer les règles inconnues, changer leur contenu ou élargir le
réseau. Un test couvrira ce profil et le refus d'une règle étrangère avant reprise.

## Preuves techniques

Mise à jour : le défaut est reproduit par un test rouge puis corrigé ; les14
tests de l'opération passent. Le code accepte les trois déclarations exactes
du profil existant et refuse une règle étrangère ou modifiée. Le formatage normal
réussit ; la relecture indépendante du petit delta précède la reprise réelle.

- Première exécution : exit1 en17.361s, verdict INCONCLUSIVE.
- Dossier `envoy-ingress-6404edc0b271419480268b699fdeb4c0` : summary seul,
  aucun journal ni ACK ; marqueurs d'opération retirés.
- Contrôles lecture seule : bail, sept preuves et identités réseau passent.
-25 policies possédées ; les trois noms absents du catalogue du vérificateur sont
  les trois NetworkPolicy de `oauth-proof/resources.yaml`.
- Les témoins Envoy et navigateur restent non exécutés ; aucun PASS global.
