# Rapport 63 — source SDK isolée vérifiée, GO code

## Plan reçu et validé

pool-9 confirme lecture des entrées et propose une source SDK dans le seul sous-module vide de son worktree isolé, branche SDK dédiée, sans patch vers un sibling. Méthode locale conforme au contrat : driver de la pile Hyper existante avec exécuteur privé enregistrant avant spawn, propriétaire indépendant du coordinateur, fence d'admission et préparation bind/TLS avant admission ; adoption via helper partagé avec échéances communes5+1s et jointures concurrentes HTTP/métier. Ce plan ne remplace pas les preuves à venir.

Lead valide l'emplacement et le raccordement, initialise uniquement ce sous-module après contrôle AIHEAD64e91dd/gitlink129dcc1/dossier vide. Après init : SDK HEAD exact129dcc1575e7dd59b4c7935de0b3794dbb7e7d07, SDKstatus propre, origin fetch/push `https://github.com/Djoe-Denne/rustycog.git`. Aucun checkout SDK utilisateur frère touché. Pas de changement de pin AI.

Statut AI après copie des inputs : seul non-tracké `.cursor/qa-plans/20261026-http-ac7-c1.md`, copie privée fournie par le lead à préserver et exclure des commits. Ne pas confondre cet artefact avec une mutation utilisateur ou une production modifiée.

## Autorisation locale et limites

GO code envoyé à la même session pool-9, sans nouvelle approbation humaine nécessaire dans le périmètre déjà autorisé. Branche source SDK dédiée et commits SDK conservés/provenance explicite ; aucun travail jetable perdu au prochain checkout. Pas de push avant GO lead du SHA vérifié, ni gitlink final avant tests/revue/publication. Toute extension backend/harness/API/protocole désactivé revient au lead avant mutation.

Production par pool-9 ; compléments QA par test-engineer après code exploitable intégré. Compilation/fmt ciblés seulement sous slotCargo Windows24jobs après préflight, lead sansCargo ; aucun Docker/IT/campagne FGA ou SQS. Aucun résultat de correctif ou clôture C-1 acquis par ce GO.
