# 149. Intégrer la livraison dans les deux branches principales

## Mandat et méthode

Après constat que la livraison restait sur ses branches dédiées, l'utilisateur
a demandé « fais le », puis « +sdk ». Intégration autorisée dans AIForAll master
et RustyCog main, sans refonte fonctionnelle, nouveau runtime ou force-push.
Les changements locaux exclus restent préservés.

L'intégration est une publication Git de branches déjà livrées, pas une nouvelle
implémentation ni une intégration de branche de worker Ensemble. Utilisation
de l'API de fusion GitHub via la CLI authentifiée, pour conserver les deux
historiques et ne pas faire de merge manuel dans un arbre local partiellement dirty.
Une divergence/conflict non résoluble automatiquement sera traitée séparément,
sans écraser silencieusement les travaux master.

## Préconditions vérifiées

SDK main41a86db est ancêtre direct de la série livrée d60a88b (12 commits).
Aucun changement divergent SDK ; la cible de pin reste exactement d60a88b,
le SHA testé. Il doit être ancêtre du main SDK après publication.

AIForAll master8cacefd et la branche mission39024b2 divergent de2/13 commits.
Deux chemins communs seulement : jobs Cargo24 identiques dans les deux branches,
et Cargo.lock avec ajouts non chevauchants (autres services d'un côté,
Telegraph/SDK de l'autre). Les autres fichiers mission devront rester identiques
au livrable déjà testé et revu ; les autres travaux master seront conservés.

## Vérification prévue, bornée

Ascendance des livrables sur les refs distantes ; gitlink mode160000 vers d60a88b ;
égalité des arbres SDK et fichiers applicatifs mission hors lock ; union des deltas
lock/configuration ; diff check. Réutilisation des preuves composant/verticale et
HTTP de la mission pour le code strictement identique. Pas de nouvelle campagne
Cargo/IT/CI globale revendiquée pour le résultat combiné avec les autres travaux.

Ce rapport est la préparation ; les SHA publiés et contrôles finaux seront
consignés dans le rapport de clôture et le ledger.
