# Premier run Linux — refus du grand-parent public

## Constat du lead, pas finding du reviewer sécurité

HEAD db16aa726ca0f44876180ac45b4922110c030c09. Source intégrée inchangée depuis le PASS statique ciblé. La revue sécurité n'avait exécuté aucun test Linux et n'avait démontré aucun défaut de sécurité ; ce nouveau constat est fonctionnel et fondé sur l'exécution du lead.

Environnement vérifié avant run : WSL2 ARTIST-Ubuntu initialement Stopped, Ubuntu24.04, x86_64, Python /usr/bin/python3. Diagnostic metadata seul en root ; tests effectivement sous utilisateur existant artist/UID1000, aucun compte créé ni configuration de distro modifiée. Pas de cible/HTTP/Kind ou binaire k6/Chrome exécuté.

Commande réelle : wsl --distribution ARTIST-Ubuntu --user artist --cd /mnt/c/Users/djden/source/repos/AIForAll --exec /bin/sh -c 'test "$(id -u)" = 1000 && uname -m && /usr/bin/python3 -B -m unittest discover -s ops/tests/k6/tests -p "test_*.py"'.

Résultat : exit1, Ran37tests, failures1/errors2, sans skips. Erreurs test_actual_process_success_nonzero_and_no_ambient_overrides et test_foreign_preexisting_child_not_stopped ; échec test_interruption_reaps_descendants_by_identity (PATH_INVALID au lieu de INTERRUPTED).

Chemin d'erreur : test_boundaries.py::PosixTests.supervise:229 -> safety.new_output:106 -> private_path:89 -> Rejected(PATH_INVALID). Le parent immédiat créé par TemporaryDirectory est0700/UID courant ; private_path(directory=True) exige aussi que son parent soit privé/du même UID. Le grand-parent usuel /tmp est public/sticky : le code refuse avant lancement du superviseur testé.

## Correctif borné attendu

Clarifier la validation des répertoires selon le contrat : le parent immédiat d'une sortie doit être privé/possédé/canonique et la sortie neuve/non liée/hors Git. Ne pas imposer accidentellement un grand-parent privé ni modifier arbitrairement la sécurité de /tmp/RUNNER_TEMP pour obtenir vert. Conserver vérifications propriétaire, droits nécessaires, résolution canonique, absence de lien et de réutilisation ; ne pas modifier l'autorité historique local_full_target.py.

Ajouter un test de régression discriminant acceptant une sortie neuve sous parent privé avec grand-parent public, plus refus de parent immédiat public/étranger/lien et sorties préexistantes. Les tests d'erreur ne doivent pas réussir sur un PATH_INVALID avant d'avoir réellement exercé l'enfant : vérifier les codes attendus pour sentinelle et overflow, pas seulement assertRaises(Rejected).

Le test préexistant qui crée un enfant témoin l'arrête dans son finally ; aucune erreur de ce cleanup n'est remontée dans le run. Ce résultat ne prouve toujours pas le cleanup du superviseur pour les cas arrêtés avant création. WSL reste sous bail parent pour cette campagne/correction ; aucun worker ne stoppe distro/Docker/Kind. Aucune ressource Docker créée.

## Ownership et suite

Même auteur pool-4/session ses_e76907ccbffet7VF3cPr8KCFXV, production et tests couplés autorisés dans ops/tests/k6/**. Pas de nouveau framework, mock HTTP, runtime ou extension de QA. Tests Windows utiles mais insuffisants : le lead rejouera les tests Linux sur le contenu effectivement corrigé. Le changement de garde privée justifie ensuite une revue delta bornée, sans refaire la revue complète par principe.
