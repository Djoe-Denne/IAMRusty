# 8 — Application autorisée, sans plafond local

L'utilisateur a sélectionné **« Redémarrer LiteLLM »** après présentation du
rapport 7 et des effets sur les appels en cours et le cache mémoire.

Action bornée : arrêter le seul listener LiteLLM après vérification de son
identité et de son ascendance au launcher possédé ; relancer via le mutex natif
ensureLiteLLM. Aucun daemon GrepAI, index, credential ou processus ARTIST à arrêter.
Le fichier config ne contient plus max_parallel_requests ; aucun remplacement
par 7 ni par un autre plafond n'est autorisé.

Le moniteur sera repris sur le même job et watcher. Il conservera les compteurs
historiques et suivra les rotations de logs du proxy. Comme ARTIST partage le
proxy, le faux pourcentage par projet est supprimé : progression par phase et
compteurs partagés seulement, 100 % réservé à la readiness native.

Prochaine preuve : /model/info sans plafond en mémoire, endpoint authentifié
disponible, mêmes identités watcher, moniteur repris. Ce rapport ne prétend pas
encore que l'application a réussi.
