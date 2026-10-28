# 10 — Enrichissement AIForAll, ARTIST et re-ff8

## Autorisation / objectif
L'utilisateur demande explicitement l'enrichissement des trois projets, après
clarification du caractère dégradé du premier cycle. Luna xhigh, proxy sans
plafond local ajouté. Trois instances GrepAI distinctes, campagnes en parallèle.

## Préflight réutilisable
Statuts natifs vérifiés : aucun watcher actif dans les log-dirs des trois projets.
- AIForAll : endpoint 3062, racine AIForAll.
- ARTIST : endpoint 3061, racine ARTIST.
- re-ff8 : endpoint 3063, racine source commune **retro-eng**, inchangée. Le corpus
  inclut le dépôt de recherche re-ff8 et son dépôt frère de réécriture ; aucune
  connexion n'est réactivée ou arbitrairement restreinte au sous-dépôt.

La source v0.37 ne propose pas de commande de reprise « résumés manquants seuls ».
Le chemin natif disponible est la reconstruction/enrichissement au démarrage du
watcher. Aucun index n'est supprimé, aucune nouvelle initialisation du corpus.

## Preuve de besoin / qualité
Audit en lecture seule du snapshot AIForAll : 2055 fichiers couverts ; 2021 résumés
de fichier manquants, ainsi que 18 area, 107 category et 1149 subcategory sans
résumé. Ce n'était donc pas seulement neuf résumés isolés manquants : les neuf
erreurs avaient interrompu des séries entières. Les chunks non reliés sont hors
compteur de résumés. Aucune estimation de coût réutilisée comme mesure réelle.

## Suivi mis en place
Moniteur paramétré par projet et nouveau dossier de campagne, sans écraser les
preuves antérieures. Heartbeat sur l'identité exacte, pas de relance d'un daemon
disparu. Vérification finale des résumés persistés via CLI en lecture seule,
contrôle de couverture de fichiers et stabilité du snapshot ; `completed_partial`
si résumés absents, erreurs ou couverture non certifiable. Pas de faux pourcentage
par projet déduit des compteurs partagés LiteLLM, pas de provenance Luna garantie
pour toutes les features par ce seul contrôle.

Fin : arrêt gracieux du seul daemon possédé par chaque campagne ; proxy global et
autres projets conservés. Aucun restart du proxy pendant l'enrichissement.

## Vérification / décision
Tests de classification de couverture ajoutés ; exécution avant admission.
Managers ARTIST/retro-eng passent à v5 pour disposer du même lifecycle testé
qu'AIForAll, sans changer les racines ni les délais historiques de retro-eng.
Aucune nouvelle question : lancement déjà autorisé. Le succès final n'est pas
encore acquis et sera évalué séparément pour chaque projet.
