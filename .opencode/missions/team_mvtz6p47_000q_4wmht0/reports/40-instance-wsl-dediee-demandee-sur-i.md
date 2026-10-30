# Nouvelle instance WSL dédiée demandée sur I:

## Changement demandé

Vous préférez une instance WSL propre au projet, nommée **AI for All**, stockée sur
le disque **I:**, avec autorisation d'y installer les outils nécessaires. Le lead
retient cette instruction : aucun paquet ne sera ajouté à ARTIST-Ubuntu et cette
ancienne distribution reste inchangée et arrêtée.

## Mise en œuvre prévue

Vérifier le disque et les options WSL disponibles, consulter la documentation
officielle, puis créer l'instance sans remplacer de dossier/distribution existants
ni changer la distribution par défaut. Ubuntu24.04 est la cible prévue, conforme
au contrat nominal ; son acquisition et son identité devront être tracées.

L'architecte doit préciser le rattachement de cet hôte dédié au contrat de
diagnostic avant sa création. Ce travail documentaire et le préflight Windows
peuvent avancer en parallèle. Aucun nouveau GO de routine n'est demandé : la
création est maintenant explicitement demandée par l'utilisateur.

## Limites conservées

L'autorisation d'installation porte sur la nouvelle instance, pas sur Docker,
les autres distributions, les réglages Windows globaux ou des ressources étrangères.
Pins, sandbox, confidentialité, propriété et cleanup restent obligatoires. Les
builds/tests Rust de développement restent natifs Windows. Aucune installation
ou instance créée n'est encore acquise à la rédaction ; la qualification navigateur
et HTTP demeure ouverte.

## Preuves réutilisées

- Préflight précédent : ARTIST-Ubuntu24.04/UID1000, bibliothèques manquantes,
  aucune installation ; distribution revenue Stopped, Docker Desktop étranger actif.
- Source k6 `1c0036f`, 140 tests Linux réussis sur22/24 au run40985953377 ; ces
  preuves ne qualifient pas une nouvelle instance encore absente.
- Le ledger contient la demande utilisateur et tiendra l'identité, le bail et les
  inventaires de la nouvelle distribution, sans adoption d'un ancien bail.
- Préflight Windows actuel : I: NTFS sain, environ1.24To libres en unités décimales ;
  le parent I:\WSL existe, le chemin prévu I:\WSL\AIForAll n'existe pas. WSL2.6.1.0
  expose les options de nom/emplacement et de non-lancement. L'aide locale est
  affichée malgré son exit255 ; aucune création n'a été tentée.
- Sources officielles relues : MicrosoftDocs/WSL/basic-commands.md et le guide
  Ubuntu WSL d'installation, qui distingue explicitement Ubuntu-24.04 de la
  distribution Ubuntu par défaut. Ce préflight ne prouve pas encore l'acquisition
  ou la création de l'instance.
