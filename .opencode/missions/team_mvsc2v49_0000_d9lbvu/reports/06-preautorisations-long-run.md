# Long run — autorisations données à l'avance dans le périmètre de la mission

## Ce qui change pour l'utilisateur

Le but reste de corriger et qualifier la sécurité de l'authentification. L'utilisateur a demandé pourquoi les accords devaient être différés et a expressément proposé de les donner à l'avance. Ces demandes provenaient du brouillon précédent : elles n'imposaient pas de multiplier les questions pendant le travail.

Recommandation retenue : enregistrer une délégation opérationnelle bornée maintenant, puis avancer sans question de routine après le start natif. Les checkpoints restent des informations, pas des demandes répétées de GO. Une impossibilité réelle hors mandat ou un conflit de sécurité obligatoire restent signalés plutôt que contournés.

## Préautorisations proposées dans cette révision

### Décisions d'architecture déjà identifiées

L'architecte reste auteur et décideur structurel pour les trois frontières du plan : séparation d'identités dans les permissions/projections, politique du signataire distant, parcours navigateur/laboratoire. Le lead reçoit délégation pour examiner et ratifier explicitement ces contrats avant leurs consommateurs, sans renvoyer systématiquement ces choix à l'utilisateur.

Ce n'est pas l'acceptation à blanc d'un futur document : alternatives, conséquences et validation sont tracées au moment où le livrable existe. Les ADR Accepted et invariants de sécurité continuent de primer. Une refonte hors mission ou un changement de cible incompatible n'est pas couvert.

### Publication du SDK nécessaire à la mission

Publication préautorisée du commit exact validé sur le dépôt canonique `github.com/Djoe-Denne/rustycog`, sur une branche de mission après vérifications/revues pertinentes et inspection du diff ; sélection du même commit dans le gitlink ensuite. Le remote origin actuel a été vérifié en lecture, sans imprimer de credentials.

Pas de force-push, réécriture d'historique, fusion distante automatique vers une branche protégée ni publication de changements étrangers. Cette autorisation n'est pas un droit général de publier tous les dépôts ou de pousser tout le travail courant de l'utilisateur.

### Infrastructure locale et preuve intégrée

Préautorisation des fixtures testcontainers existantes et du laboratoire `kind-aiforall-local-full` : préflight, démarrage/réutilisation ou création si absent selon les conventions du dépôt, images Linux de livraison, déploiements de test nécessaires, diagnostics utiles, campagne et arrêt gracieux possédé.

Les corrections et IT finales précèdent toujours Kind. Les identités exactes, inventaires, baux, ressources exclusives, inspection des effets transitifs et cleanup restent des conditions d'exécution, mais plus des demandes d'accord de principe. Une ressource de propriété inconnue ou un remplacement destructif ne sont pas autorisés par cette proposition.

## Ce qui ne change pas

- Objectif authentification ; pas de chantier plugin Ensemble, production ou infrastructure étrangère.
- OpenAI/Sol et ses variantes natives, seuil de départ 50, classification désactivée.
- QA proportionate : les mêmes quatorze commandes de référence et douze critères, ajustés pour désigner la délégation plutôt qu'une autorisation ultérieure. Aucune campagne ou chaîne de reviewers ajoutée.
- Contrats écrits/revus/ratifiés puis intégrés avant consommateurs ; séparation des worktrees, un Cargo Windows/24 jobs et vérifications réelles.
- Protections natives des outils, des secrets et des changements utilisateur ; pas de contournement de permission ni reprise d'une pause opérateur sans son GO distinct.
- Pas de cible protégée, prune/reset/suppression destructive, exposition publique ou nouveau service payant implicitement autorisé.
- L'approbation de la politique par start reste un verrou natif utilisateur. Le superviseur ne l'invoque pas à sa place.

## Diff de préparation et état

Le brouillon modifie trois critères d'acceptation (ratification, runtime, publication) et précise les instructions long_run. Les tâches importées et leur graphe ne changent pas ; aucun modèle n'est substitué. Les quatre racines restent pending, sans worker.

Cette révision a été représentée par config, avec confirmation native de politique proposée, exécution paused/not approved. Le précédent hash ne la couvre plus. La préautorisation prendra effet avec le start utilisateur de cette version présentée, pas avec l'édition documentaire ni la commande config.

Les autres options auraient été de conserver les accords séparés du brouillon précédent (davantage d'interruptions), ou de ne pas démarrer (sources/runtime inchangés et risques toujours ouverts). Aucun délai ni coût total n'est établi. L'autorisation générale n'est pas assimilée à une dispense de qualité ou de sécurité.

## Preuves et limites

Source utilisateur : « Je peux te les donner à l'avance. Tu as le droit de tout faire. » Portée traduite explicitement dans le brouillon de cette mission, pas dans des permissions globales.

Références relues : règle infra-safety, skill lifecycle et procédure d'acceptation des ADR (revue explicite). Le contexte canonique de laboratoire est celui du contrat 0412. Les éventuelles autorisations historiques de destruction ne sont pas utilisées pour élargir cette proposition.

Lecture locale du remote SDK : origin correspond à github.com/Djoe-Denne/rustycog.git. Aucun accès réseau à ce dépôt, push, changement de code, test, worker ou runtime n'a été effectué pour cette correction.

Validation documentaire : JSON de politique valide, quatre variantes Sol et seuil 50 conservés, QA proportionate avec quatorze commandes et douze critères, onze instructions long_run ; cinq fichiers relus sans caractère de remplacement/espace final ; `git diff --check` exit 0. Commande native config exit 0, puis proposition reçue sous le hash `df8486d1c5c18ff89fbb46fa984c02f49259fe6ff307c56b21c182e89809602e`. Aucun start invoqué, aucun changement de politique après présentation.
