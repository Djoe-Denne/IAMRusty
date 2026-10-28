# 27 — Retour au mode local vérifié

## Résultat
Les sept profils GrepAI précédemment migrés utilisent maintenant
**`rpg.feature_mode: local`**. Embeddings Ollama existants conservés, racines et
index séparés inchangés, notamment la racine commune retro-eng pour re-ff8.
Les trois modèles publics existants sont aussi en mode local dans l'arbre de
travail ; aucun changement de staging effectué.

Les paramètres LLM et credentials restent présents mais inactifs. `ensure.mjs`
ne démarre désormais LiteLLM que pour un mode explicite llm/hybrid avec endpoint
proxy configuré ; le mode local n'effectue ni sonde, ni auth, ni lancement.
Le proxy déjà en mémoire et son cache ne sont ni arrêtés ni vidés.

## Vérification à 20:19:52Z
- **8 tests PASS**, dont local avec endpoint dormant : zéro sonde/auth/lock/lancement.
- Vérificateur de déploiement : **7 profils PASS**, config privée ignorée par Git,
  absence de credential dans les modèles publics ou le diff staged.
- Sept catalogues MCP rechargés, **14 outils chacun**, managers
  **`--rpg-local-v7`**. Aucun appel d'outil GrepAI utile, aucun watcher démarré.
- Les **sept graphes ont exactement leur SHA256 d'avant changement** ; credentials
  également inchangés. Inventaire OS final : **zéro watcher GrepAI**.

Preuves : `C:/Users/djden/AppData/Local/LiteLLM/local-mode-before-20261023.json`,
`local-mode-tests-20261023.json`, `local-mode-verification-20261023.json`,
`local-mode-final-proof-20261023.json`.

## Limites / continuité
Aucun enrichissement réel lancé pour vérifier le retour local : cela aurait
reconstruit les graphes. Le prochain appel utile réactivera normalement le watcher
et BuildFull, **en extraction locale**, pas via Luna. Le problème de reconstruction
au démarrage n'est pas corrigé ; les quatre sauvegardes AIForAll sont conservées.
Le binaire Windows anti-popups reste installé, sans reconstruction ni remplacement.

Localisation : configs privées/publics, helper d'autostart et tests, vérificateur,
sept marqueurs MCP ; aucun code applicatif AIForAll modifié pour cette opération.
Pas de Docker/Kind/WSL/Cargo ni ressource Docker créée, aucun commit/push/reset.
