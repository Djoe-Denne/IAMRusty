# Avis borné : durée de protection du bundle si un enfant survit

Architecte12 existant, même session/profil/modèle. Aucun code/runtime/test ou
permission modifié. ContratsWindowsAccepted, révision2, QAproportionate inchangés.

## Problème nouveau précis

L'auteur20 rapporte S-1créationexclusive/cleanup parhandle implémenté et testé,
R-1bundle fermé/entréesauthentifiées/pins jusqu'au finally implémenté. Source finale
pas encore recoupée par lead : livraison native10fichiers/delta83262octets annoncée,
mais le dernier retour reçu ne contient que HANDOFF, retransmission demandée.
Ne pas supposer sa correction acceptée. Référence actuelle68631c0 pour Child,
demeuré inchangé selon l'auteur ; revue complète21 déjà persistée.

Le Child existant peut rendre Cleanup=false avec un enfant survivant après attente
bornée et arrêt gracieux. L'auteur constate que le finally libère alors les leases
de code. Ne pas appeler Dispose ne résoudrait pas la sortie du hostWindows, qui
fermerait ses handles. Un résultat nonPASS ne constitue pas une protection du code
durant la vie résiduelle du survivant. Le vrai navigateur reste gelé.

## Décision nécessaire dans les contraintes existantes

Évaluer le plus petit moyen conforme de conserver les garanties de R-1 et le sens
exact de cette garantie après échec de cleanup. Examiner aussi la frontière de
confiance des ACL de chaque fichier, pas uniquement le répertoire, et les garanties
Windows effectives de code image chargé. Ne pas conclure que refuser PASS suffit.
La revueR-1 vise substitution de code livré, dont DLL différées ; vérifier à quel
acteur une modification resterait possible une fois les handles relâchés.

Contraintes : un seul parent existant, pas deuxième superviseur/service/framework,
attente indéfinie, force/killonclose/TerminateJobObject, élévation, mutationACL
existante, nouveauvolume/VM/Docker/WSL ou relaxation parfacilité. Ne pas promettre
un héritage de handles à tous descendants sans vérifier les politiques de création
Go/k6/ChromiumWindows. Ne pas changer un contratAccepted/QAJSON ou activer un run.

Comparer options réellement raisonnables : détail compatible de cycle de vie ou
preuve immutabilité/code nonmodifié jusqu'à fin, limitation explicite de lancement
si aucun chemin conforme, ou plus petite frontière à arbitrer si indispensable.
Un maintien borné du parent existant ne doit pas être présenté comme une garantie
indéfinie ni un deuxième superviseur caché. La fin brutale de tout parent est une
limite distincte, pas une permission de masquer un survivant connu dans le chemin
normal de timeout. Rechercher sourcesofficielles avantchoix important.

Si la version finale est indispensable pour trancher, indiquer les symboles ou
extraits minimums : lead les fournira après récupération/intégration, pas enquête
worktree externe. Avis technique avec faits/hypothèses/limites et, uniquement si
nécessaire, petit Proposed ; pas réécriture du framework ni dispositif de preuve
supplémentaire. Les versions/inspectk6 sansChrome restent des travaux indépendants
envisageables après leurs propres préconditions, pas une qualification navigateur.
