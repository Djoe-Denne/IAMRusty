# Récupération native des sources des deux auteurs avant nettoyage

Objectif : sauvegarder les livrables historiques utiles des copies pool-13 et
pool-20, sans réintroduire leurs anciennes protections ni modifier le code publié.
L'utilisateur a autorisé le retrait des dix copies uniquement après sauvegarde
récupérable vérifiée. Aucun retrait si cette récupération est incomplète.

Le lead ne relit pas les contenus de ces deux arbres : des lectures externes ont
été refusées auparavant. Le transfert natif par l'auteur de ses seules sources
publiques est l'alternative déjà utilisée au ledger, pas une ouverture des droits
du lead ou un nouvel outil de lecture du chemin refusé.

## Travail de récupération, pas une reprise du développement

Même membre, session, arbre et modèle. Aucune modification des sources, aucun
commit, test, outil k6/Chrome, WSL, Docker, collecte ou nouvelle investigation.
Ne pas modifier de permissions ni produire un nouveau fichier métier.

Depuis votre propre arbre, exporter en mémoire les fichiers publics modifiés ou
non suivis identifiés par git status. Pour pool-13, inclure les sept scripts publics
`.tmp/*20261030.py` et `.tmp/*20261030.ps1` du dernier inventaire ; ils sont des sources
ponctuelles, pas des données d'exécution. Pour pool-20, EXCLURE le journal privé
`.tmp/k6-windows-unit-fixtures.jsonl`. Exclure tous les autres logs, profils,
configurations privées, baux, caches, outils et binaires.

Rendre par résultat natif final un paquet ZIP en mémoire encodé Base64, un manifeste
avec chemin relatif, taille et SHA256 de chaque fichier, SHA256 du ZIP, HEAD de base
et constat du dossier rustycog vide ou non. Pas de symlink suivi. Ne pas écrire dans
le checkout principal. Une archive de sources n'est pas une qualification ni une
ratification. Si cette transmission est interdite ou impossible, rendre le blocage
précis et conserver l'arbre ; ne pas essayer d'autres permissions ou lecteurs.

Le lead vérifiera les octets livrés, restaurera les fichiers hors des arbres puis
enregistrera la disposition des anciennes versions avant toute suppression.
Source de continuité : `.cursor/handoffs/20261029-k6-01-socle.md`, rapport102 ;
politique approuvée révision3 inchangée. Cette opération ne relance pas les tâches
Linux différées et n'autorise aucune expérience suspendue.
