# Les services démarrent, le trajet d'authentification reste à qualifier

IAM, Envoy et Manifesto ont maintenant un Pod prêt. La revue infra indépendante
confirme la portée limitée des reprises et la provenance de l'image Manifesto.
Les données, anciennes configurations et autres images ont été conservées.

La requête de préparation du test de certificat n'a toutefois pas reçu de réponse
dans le délai prévu. Aucun certificat n'a été changé : ce n'est pas un échec du
contrôle de nom, mais un trajet HTTP encore non qualifié. La connexion au port
d'entrée réussit et les destinations Kubernetes sont prêtes.

Le diagnostic a aussi confirmé que l'image Envoy ne contient pas l'outil curl
sur lequel reposait la lecture de ses compteurs internes. Je n'installe rien et
ne remplace pas cette lecture par des zéros. L'architecte examine la plus petite
lecture locale conforme réutilisant le mécanisme de transfert existant ; son
contrat devra être ratifié avant tout changement de cette garde.

La preuve navigateur reste distincte : la connexion actuellement exposée ne
garantit pas un profil isolé. L'opération d'exécution de code non autorisée n'a
pas été utilisée, et aucun client HTTP n'est présenté comme navigateur.

## Preuves et limites

- 15 IT ciblées et cinq unités Manifesto réussies, preuves réutilisées.
- Reprises Envoy et Manifesto exit0/Ready ; contrôle infra borné, pas PASS global.
- TCP18080 ouvert ; Services et EndpointSlices cohérents, pas preuve du transfert.
- Lecture Admin : échec natif1, diagnostic fermé executable file not found pour curl.
- Preuves privées : auth-envoy-san-preflight-20261029.json et
  auth-envoy-admin-frontier-20261029.json ; aucun journal ou secret brut publié.

## Décision nécessaire pour le futur test navigateur

Le parcours final exige un vrai navigateur isolé. Le pilote installé est attaché
à un contexte partagé ; ses opérations ordinaires ne permettent pas d'en créer
un autre. L'opération `browser_run_code_unsafe`, encore non autorisée, permettrait
de créer un contexte neuf, d'exécuter le scénario existant relu, puis de le fermer.
Elle permet techniquement du code dans le processus du pilote, pas seulement
dans une page : c'est une capacité sensible, pas une simple ouverture d'onglet.

Recommandation : autoriser uniquement cet usage borné pour cette campagne,
création et fermeture du contexte comprises. Aucun installateur, changement de
configuration globale, profil personnel, export de jeton/cookie ou capture brute.
Les permissions natives restent applicables ; un refus d'outil restera bloquant.

Autres choix : conduire le navigateur manuellement dans un profil isolé fourni
par l'utilisateur, avec collecte expurgée des preuves ; ou maintenir le refus et
reporter cette preuve. Dans ce dernier cas les correctifs et leurs résultats
restent acquis, mais la mission ne peut pas être déclarée entièrement qualifiée.
Durée et effort de conduite manuelle non établis. Aucune de ces options ne
supprime le diagnostic HTTP et le contrôle de certificat encore nécessaires.
Ces travaux indépendants continuent sans attendre une autorisation navigateur.

## Décisions reçues et préparation

L'utilisateur a autorisé **ce seul scénario navigateur borné**. Il n'a pas autorisé
un usage général de l'outil sensible. L'enveloppe préparée crée son propre contexte
vide et le ferme systématiquement ; la fonction du scénario existant y est
strictement identique. Syntaxe vérifiée, aucun navigateur encore exécuté.

Le contrat architecte de lecture Admin a été ratifié avant code. Son risque est
explicite : le bref transfert loopback expose l'interface complète aux processus
du poste de confiance, même si notre fonction ne fait que des GET fermés. Il ne
sera jamais actif pendant la navigation OAuth. L'implémentation est isolée et
sera testée et revue avant toute lecture réelle ; aucun changement d'image Envoy.
