# Lecture Admin : tests locaux réussis, accès réel encore suspendu

La lecture spécialisée des compteurs Envoy est intégrée. Ses 40 tests passent
dans le dépôt canonique, dont les cinq nouveaux scénarios de refus, d'identité,
de limites, de compteurs absents et de fermeture de l'enfant possédé.

Ces tests simulent le réseau et les processus : ils ne prouvent pas encore les
compteurs réels ou le contrôle du certificat. La revue sécurité du delta intégré
doit précéder cet usage réel. Aucun outil n'a été installé dans Envoy et son
image reste inchangée.

Une piste réseau concrète est également examinée : la règle d'entrée d'Envoy
déclarée n'autorise qu'un client interne au cluster, alors que le navigateur entre
depuis l'hôte. L'effet réel de cette règle et de la translation d'adresse reste à
vérifier. Aucune règle n'est supprimée ou élargie pour essayer au hasard.

Le pilote navigateur est déjà installé, mais son navigateur n'est pas démarré.
Le moment venu, un profil temporaire possédé sera utilisé, sans profil personnel,
installation ou modification globale. L'autorisation sensible reste limitée au
scénario existant dans un contexte isolé, puis à sa fermeture.

## Preuves

- Suite du candidat : 40 PASS ; suite intégrée : 40 PASS, aucun ignoré.
- Source intégrée identique au candidat après normalisation des fins de ligne.
- Aucun Cargo, conteneur, accès Admin, certificat ou navigateur lancé par ces tests.
- Contrat ratifié : local-full-envoy-admin-readonly.md ; revue indépendante en cours.

## Régression supplémentaire détectée avant usage réel

La comparaison avec le client officiel Kubernetes a révélé un défaut que les
premiers tests simulaient mal : ses notifications indiquent le port local choisi,
pas le port distant d'Envoy. Le lecteur aurait refusé une réponse pourtant légitime.

Le test enrichi échoue sur l'ancien parseur, puis les 40 tests passent après le
correctif limité à cette comparaison. Les ports étrangers restent refusés.
Deux tests supplémentaires de l'adaptateur de preuve vérifient que compteurs
manquants, remise à zéro et changement d'époque de redémarrage ne produisent pas
un succès. Les contrôles d'identité restent couverts par la suite de la garde.
La nouvelle photo du code est transmise à la revue ; aucun accès réel n'a été fait.
