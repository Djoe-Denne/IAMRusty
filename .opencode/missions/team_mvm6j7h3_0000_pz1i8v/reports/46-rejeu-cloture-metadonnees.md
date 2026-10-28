# 46 — Clôture du nouveau scénario : métadonnées seulement

## Résultat
Audit des traces terminé :3sessions OpenAI réussies,1session Haiku bloquée par
quota mensuel Go (429),0token/0outil pour cette dernière. Aucun retry/fallback.
Le bilan détaillé reste hors checkout et est remis par lien à l'utilisateur ;
ce rapport ne reproduit pas les parcours métier ni leurs conclusions techniques.

## Intégrité et usage des outils
- Même nouveau prompt914caractères, corps inchangé, hash livré commun
  34b775d6fd52dd6091a7c2125e19c7ed1ccd0ce829b5ba02cc6bab1edb9cef27.
- Modèles/variantes conformes, agentbuild, aucunparent,1user chacune. Pagination
  complète2pages dont dernièrevide. Captures réutilisées pour l'audit.
-24fichiers suivis stables aux4départs/aprèsfins ; snapshots sans changement.
-12anciensaudits toujours absents, archives auhash attendu, aucun nom de ces
  anciens fichiers dans les sorties examinées. Pas d'isolement absolu des index.
- Recherches Rust MCP réellement vérifiées, profil local-qwen3-8b : Luna high2,
  Luna xhigh5, Solmedium1. Alias dans les scripts pris en compte manuellement.
- Serena utile chez les3 (définitions reçues, pas seulement initialisation).
  GrapheHIR et nouvelle bibliothèque locale non exercés. ContextMode partiel
  chez Sol, absent chez les2Luna ; aucun changement de consigne pour forcer l'usage.

## Mesures descriptives
- Luna high :101,998s,27appels directs,19messagesdont16assistant.
- Luna xhigh :381,119s,88appels directs,27messagesdont24assistant.
- Sol medium :388,533s,87appels directs,52messagesdont49assistant.
Sujet et chemins retenus différents, concurrence/cachepartagés : pas de benchmark
isolé ou d'économie financière démontrée. Haiku n'entre pas dans ces comparaisons.

## Clôture
4CLI terminés. Les anciens audits restent archivés, nouveaux détails hors dépôt,
aucune session supplémentaire ni appelGo. Aucun codeapp modifié, test/buildapp,
Cargo, Docker/Kind/WSL, ressourceDocker, purge/restartMCP ou déchargement8B.
Pas de commit/stash/reset/merge/push. Réponse fondée sur appels/résultats explicites,
pas sur les champs de raisonnement interne. Aucun blocage de collecte restant.
