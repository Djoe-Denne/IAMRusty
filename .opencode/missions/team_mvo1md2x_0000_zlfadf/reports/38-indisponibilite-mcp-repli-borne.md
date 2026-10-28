# Rapport 38 — outils MCP absents après restart : décision de repli

## Objectif et état conservé

Corriger l'idempotence Sentinel dans l'exception autorisée, sans réduire AC1–AC7. Contrat09a59ee remis et lu par la même session t3-runtime. HTTP16PASS, résilience2PASS et runtime4PASS conservés, aucun rerun de reprise. AC6 rouge réel reste à corriger ; aucun Cargo/runtime lancé depuis le restart annoncé par l'auteur.

## Indisponibilité vérifiée

L'auteur constate absence d'exposition de ctx_execute et Serena dans son catalogue. Le lead vérifie à son tour : search namespace `context-mode` retourne items[]/remaining0, recherche Serena find_referencing_symbols idem. Recherche libre ctx_execute ne retourne qu'un outil opencode non pertinent ; cela ne fournit pas Context Mode. Ce n'est ni une erreur de script, ni une preuve de panne du serveur applicatif ou de Docker. Aucun plugin réinstallé, index reconstruit ou backend redémarré.

## Options et recommandation

1. **Recommandé : autoriser pour cette mission le repli borné sans Context Mode**, tant qu'il reste absent. Shell Windows natif, logs complets persistés localement ; extraction locale filtrée et plafonnée des seuls diagnostics, noms/comptes de tests, codes de sortie et preuves décisives. Pas de dump brut dans le contexte. Conserver chemins et logs récupérables, erreurs d'analyse distinctes des erreurs de test. Sources ciblées et patchs contrôlés pour l'absence Serena, déjà permis par la politique de fallback. Un Cargo à la fois,24jobs configurés ; seulement tests ciblés du classifier puis routing IT existante et contrôles déjà convenus, nouveau préflight/bail avant runtime. Aucun nouveau harness ni baisse d'oracle. Revenir aux MCP s'ils sont de nouveau exposés, sans double exécution.
2. **Attendre la restauration de l'exposition MCP** : pas de compilation/test/analyse de logs sans Context Mode ; préparation documentaire et lectures ciblées possibles. Pas de réparation globale ou installation automatique par les agents.

## Décision demandée et limite

La politique du dépôt exige un accord humain explicite avant poursuite sans Context Mode réellement indisponible ; l'autorisation Sentinel et le mode long_run ne la suppriment pas. Choisir ce repli limité ou attendre les MCP. Pas de nouvelle autorisation de périmètre fonctionnel sollicitée, ni de nouveau résultat vert revendiqué.
