# Nettoyage : k6 seul pour les E2E

## Décision utilisateur du 6 novembre 2026

Après arrêt explicite du run Python, l'utilisateur demande de retirer les anciens
lanceurs E2E hors k6 et de nettoyer toutes les branches locales et copies de
travail secondaires, en rapatriant les livrables utiles sur la branche principale.
Ici cette branche est `master`, conservée sans renommage. Cette décision remplace
l'obligation de conserver les anciens pilotes dans le contrat k6 du 31 octobre ;
elle ne modifie pas les oracles métier ni ne qualifie leur portage.

## Code retiré et code conservé

Retrait de `ops/tests/local-full-acceptance.py`, `local_full_e2e.py` et leurs deux
suites unitaires propres ; retrait de `ops/tests/oauth-browser-local-full.js` et
de son seul test source ; retrait des quatre scripts `ops/scripts/mesh-authn-e2e*`
et `mesh-authn-kind-e2e*`. Les commandes Python de smoke et route-matrix qui
appartenaient au même ancien pilote ne sont plus des interfaces disponibles.

Conservation des scénarios, parents, adaptateurs et tests de `ops/tests/k6/`,
notamment ses auxiliaires Python. Conservation des tests unitaires/IT applicatifs,
du runtime, des fixtures utiles, des certificats, secrets et données. Le helper
de cible et ses tests de tunnel restent conservés : supprimer le pilote E2E ne
donne pas permission de retirer une protection ou un outil partagé.

Les anciens contrats, matrices et rapports restent des archives de connaissance,
pas des commandes exécutables ni des preuves valables pour une nouvelle campagne.
Les points d'entrée actuels sont décrits dans `ops/tests/k6/AUTH-WINDOWS-PARENT.md`.

## Limites explicites de couverture

Le code k6 OAuth livré est encore câblé sur GitLab et le fournisseur synthétique.
Il n'est pas raccordé au profil GitHub seul par ce nettoyage. Aucun test k6 réel
n'est lancé ici. Les 39 obligations mesh et six Apparatus restent un inventaire
historique à rapprocher du circuit retenu : leur suppression du pilote Python
ne vaut pas portage, réduction silencieuse du décompte ou PASS. Les six Apparatus
étaient déjà explicitement non raccordés dans ce pilote.

Le run original a été interrompu à la demande de l'utilisateur. Son accord, sa
consommation, son reçu et son verrou sont préservés. Il n'a pas de bilan final
ni de terminal RETAINED durable prouvé ; aucun réarmement automatique n'est admis.

## Préservation et nettoyage Git

Avant retrait : sauvegarde externe vérifiée des 140 branches locales, des patchs
et des fichiers non suivis de 34 arbres, sans refus de sauvegarde. Le bundle Git
a été vérifié, avec couverture de chaque référence. Aucun contenu de credentials
ou de certificat d'exécution n'est exporté dans cette documentation.

Le tri a identifié 12 commits distincts non ancêtres de master, sur 19 références
historiques : ils doivent être rapprochés du code intégré plutôt que fusionnés
aveuglément. Les ajouts non committés de l'ancienne activation additive sont
abandonnés par la décision GitHub seul ; ne pas les réintroduire.

La purge native des archives Ensemble est actuellement bloquée par des anciennes
associations de branches hors de leur espace de conservation. Pas de contournement
par suppression manuelle de références protégées. Nettoyage Git non terminé tant
que ce blocage et la disposition des copies de travail ne sont pas résolus.

Le changement Obsidian préexistant reste préservé ; il n'est pas effacé ou mélangé
silencieusement au commit de nettoyage.

## Vérification du retrait des anciens moteurs

Les 11 tests restants de `test_oauth_browser_alignment_source.py` passent après
retrait du seul test consacré au navigateur autonome supprimé. `git diff --check`
réussit. Aucun build Rust, E2E ou démarrage de runtime lancé. Le retrait ne constitue
pas un nouveau PASS des anciennes suites maintenant supprimées.
