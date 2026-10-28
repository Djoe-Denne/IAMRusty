# Comprendre le choix avant de poursuivre

## Ce que cette mission doit garantir

Lorsqu'AIForAll crée une notification, elle doit conserver durablement l'intention de la transmettre. Si la messagerie tombe en panne, cette intention ne doit pas être perdue : l'application doit pouvoir réessayer puis reprendre correctement. La notification doit aussi devenir accessible à la bonne personne, sans ouvrir l'accès aux autres utilisateurs.

Une autre partie du travail concerne l'arrêt propre du serveur : une requête encore en cours ne doit pas continuer à modifier l'état après que l'application a annoncé son arrêt. Les défauts trouvés sur ce chemin et sur la compatibilité HTTP ont été corrigés et revus par un autre agent.

## Pourquoi nous sommes maintenant bloqués

Les vérifications ciblées de ces corrections passent. Un test d'intégration simulant une panne de messagerie puis sa reprise a cependant échoué. Nous n'avons donc pas validé l'ensemble de la mission et n'avons pas publié la nouvelle version du SDK ni enregistré sa référence finale dans le projet.

L'échec porte sur une date de prochaine tentative. Le code prévoit normalement un délai avant de réessayer. Mais le test attend aussi une série d'observations, jusqu'à douze secondes, avant de vérifier que cette date est encore dans le futur. Une date correctement fixée cinq secondes après l'échec peut déjà être passée quand le test la lit. Le test peut donc être trop exigeant à cet instant, sans que le mécanisme de reprise soit défectueux.

Ce n'est toutefois pas encore l'explication prouvée de cette exécution : le journal ne contient pas les valeurs exactes de la ligne enregistrée. Un vrai défaut de calcul ou d'enregistrement reste possible. Les preuves détaillées et le cleanup sont conservés dans les rapports97/98 ; aucun échec n'est requalifié en réussite.

## Choix 1 — autoriser un diagnostic limité, recommandé

Ajouter uniquement des observations dans ce même test, puis l'exécuter une fois. Elles permettront de comparer la date enregistrée juste après l'échec et celle relue plus tard.

La production, l'assertion actuelle, les délais et le comportement de nouvelle tentative restent inchangés. Ce choix n'autorise ni une correction automatique, ni un assouplissement du test, ni une publication. Le test pourra encore échouer : l'objectif est d'en comprendre la raison.

Cela demande une exécution supplémentaire avec les dépendances locales déjà utilisées. Sa durée exacte n'est pas garantie. Un nouveau contrôle de l'environnement précédera le run ; seules les nouvelles fixtures possédées seront arrêtées selon le bail, sans toucher aux ressources préexistantes. Le résultat peut nécessiter une décision ultérieure si la cause reste incertaine.

## Choix 2 — conserver le blocage

Aucune instrumentation ni nouvelle exécution. Les corrections et preuves déjà acquises sont conservées, mais le scénario de panne/reprise reste non validé et sa cause demeure incertaine. Nous pouvons rendre un état partiel honnête ; publication et référence finale du SDK restent bloquées dans le mandat actuel.

## Recommandation et décision demandée

Je recommande le diagnostic limité : il doit fournir les valeurs qui manquent avant de décider s'il faut corriger le test ou la production. Cela évite autant de masquer un vrai défaut que d'appliquer un correctif inutile.

L'utilisateur a demandé cette explication, **pas encore autorisé le diagnostic**. La question est reposée après ce contexte. Aucun nouveau run ou changement n'a été effectué depuis la demande d'arbitrage.
