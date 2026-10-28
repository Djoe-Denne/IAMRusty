# Rapport 33 — wrapper inspecté, GO IT T3 borné

## Inspection réelle du lead

Lecture des267lignes de `.tmp/t3-inputs/run-t3-it.cjs` dans le worktree auteur. Empreinte annoncée par auteur : `a60b0694e46d4cc70bd4d29359dc6d03ee5f72867a4176f72e291c2dcf708ff7` ; syntaxe node --check annoncée exit0, non rejouée par le lead.

Contrôles source : root relatif au worktree ; quatre cibles exactes13/192 ; images déjà présentes requises234–237, pas pull/build ; inventaires validés45–65 ; inspect échoué n'est absence que sur no-such-object ET inventaire réussi67–81 ; provenance fixture run/process/fullID et exclusion préexistants88–123 ; volumes observés UNKNOWN/Name/ask83–87 ; registres locaux multi-jours et blocage des autres baux125–150 ; cleanup uniquement après exit Cargo et sur IDs tracés152–190 ; stop gracieux et retrait Exited0 sans force/volumes ; erreurs par identité conservées ; logs/commandes/exit/signal191–225 ; inventaire final et rapport incomplet explicites251–265.

Quatre Cargo séquentiels, cleanup à la fin de chaque binaire : justification réelle = nom fixe SMTP existant et libération des fixtures entre processus, sans adoption du prédécesseur par nom. Aucun Cargo concurrent,24jobs internes configurés. Une cible échouée ou un problème opérationnel empêche le lancement des suivantes ; les cibles non exécutées restent explicitement absentes, pas PASS implicite.

## Décision

Gate wrapper levée pour ce script et ces quatre cibles uniquement. Bail31 activable après inventaire frais du script et confirmation des préconditions ; aucune rétention spéculative. Le node40201... et tout container préexistant sont exclus du set possédé. Volumes108 préexistants non adoptés, toute création opaque ask ; aucun cleanupglobal, suppressionvolume, force, Kind ou buildappDocker.

Scope opérationnel et preuves inchangés : watcher de ledger fixture existant, pas témoin Engine indépendant. La récupération T2 et cette adaptation ne valent pas nouveau harness. Toute nécessité de modification framework/backend ou d'extension de cette frontière doit être remontée avant poursuite.

## Limites et retour attendu

Pas de robustesse face à kill OS, panne brutale disque ou interruption du processus revendiquée. Le run ne prouve encore rien : logs finaux à recouper avec counts/états SQL/transport/FGA/HTTP et sources réellement exécutées. Avant run, archiver HEAD/diff et hash du script pour attribuer les preuves au contenu, sans commit forcé ni hook bypass. En fin, remettre tous résultats et cleanup, y compris en échec ; volumes inconnus conservés et nettoyage bloqué signalé. Aucun Cargo/runtime lancé par le lead, aucune IT encore déclarée PASS.
