# 9 — Sans plafond local ; campagne terminée avec avertissements

## Résultat effectif
- Proxy redémarré sur accord explicite : ancien listener 47400 arrêté après
  contrôle de commande, date de création et ascendance au launcher possédé.
  Nouveau launcher 24484, 2026-10-23T16:40:37Z ; autostart/runtime à revérifier
  avant tout prochain arrêt. OAuth et données persistées conservés.
- `/model/info` authentifié confirme `max_parallel_requests`, `rpm`, `tpm`
  absents/null et raisonnement `xhigh`. Aucun remplacement par 7 ni autre cap.
- Sept profils vérifiés PASS, clé locale non publiée. La règle d'absence de
  plafond ajouté est aussi vérifiée par `verify-deployment.mjs`.

## Concurrence GrepAI, réponse à la question utilisateur
GrepAI v0.37.0 : `config/config.go` et `embedder/factory.go` confirment que
`embedder.parallelism` est utilisé par l'embedder OpenAI, pas Ollama.
`RPGConfig` ne contient pas de paramètre de parallélisme ; `rpg/indexer.go`
parcourt fichiers/symboles séquentiellement et `rpg/summary.go` résume par niveaux.
Plusieurs projets peuvent maintenant appeler le proxy sans le cap local retiré.
Aucune option fictive ajoutée, aucun binaire GrepAI modifié.

## Campagne / preuve finale
- Même watcher AIForAll **39740**, même génération : aucun rebuild relancé lors
  de la reprise du moniteur. Premier appel MCP en warmup déjà réussi en 4,1 s.
- Ancien moniteur terminé en erreur sur rotation du proxy (notification shell),
  puis reprise du même job par moniteur **16304** : compteurs conservés,
  rotation enregistrée, faux pourcentage par projet supprimé (trafic ARTIST).
- Notification du shell `sh_169a37587001SgGqJFIc2jzDHU` : fin native à
  `2026-10-23T16:41:26Z`, durée observée ~10m45. Log `[RUNNING] ... steady` à
  18:40:38 local ; **24142 nœuds, 62227 arêtes**.
- **9 échecs de résumé**, circuit breakers et repli local : les messages
  `connection refused` sont contemporains de la coupure autorisée du proxy à
  18:40:36 local. Cycle terminé, mais pas d'enrichissement Luna intégral prouvé.
- Compteurs de proxy partagés : 213 HTTP200 / 17 erreurs avant la reprise.
  Ne pas les attribuer à AIForAll seul, ni les convertir en tokens facturés.
  L'estimation préalable 10–50M n'est pas validée par cette campagne dégradée.

## Vérification et cycle de vie
- 24 tests ciblés PASS avant lancement ; 3 tests moniteur PASS après modification.
- Vérification finale des sept profils et de l'absence de cap PASS.
- Moniteur fini, lock retiré. Stop natif gracieux du seul watcher39740 après
  contrôles d'identité/propriété. `cleanup.json` contient la preuve non secrète.
- `grepai status` après arrêt : 2055 fichiers, 10583 chunks, index 64,5 Mo lisible.
- LiteLLM conservé pour l'usage global demandé ; ARTIST et ses ressources intacts.
  Aucun Docker/Kind/WSL, aucun conteneur ni build applicatif. Aucun commit/push.

## Localisation / impact / limites
Localisation : scripts globaux lifecycle et LiteLLM, mcp.json AIForAll v5,
README LiteLLM et skill GrepAI ; canon détaillé dans le ledger de handoff.
Impact : disponibilité MCP découplée du warmup, concurrence du proxy non plafonnée
localement, moniteur anti-redémarrage avec suivi des rotations. Une instance et
un index GrepAI par projet maintenus.
Limites : qualité partielle du RPG, pas de compteur natif par nœud en daemon,
pas de mesure réelle des tokens, parallélisation RPG non configurable dans v0.37.
Pas de nouvelle campagne relancée automatiquement pour masquer les avertissements.
