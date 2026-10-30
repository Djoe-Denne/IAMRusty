# Relecture complète : une frontière de sécurité restante

L'objectif reste de vérifier k6 et Chrome sous Windows. Le rapport indépendant
complet est récupéré et recoupé avec les sources intégrées et les22tests ciblés
déjà exécutés. Il confirme la fermeture du risque de nettoyage du mauvais dossier
et du contournement du contrôle des bibliothèques Chrome déjà altérées.

Le parent ne peut plus annoncer une isolation Chromium réussie à partir de son
seul témoin partiel. **Chromium reste INCONCLUSIVE.** Cela n'est pas une preuve
d'évasion ni une désactivation de ses protections.

Il reste une frontière bloquante : conserver la protection des fichiers de code
si un processus survit après l'arrêt gracieux ou si le lanceur disparaît. La revue
confirme qu'un verdict d'échec ne suffit pas à garantir cette conservation.
L'architecte examine la disposition minimale compatible ; aucun nouveau superviseur,
droit privilégié ou arrêt forcé n'est autorisé par cette revue.

**Les essais réels restent gelés.** Aucun test n'a été répété pour récupérer le
rapport, aucun k6/Chrome Windows réel ou GET n'est encore exécuté, et aucune
publication n'est déduite des fermetures partielles.

## Références et reprise

- [Relecture indépendante complète](../../../../.cursor/review-briefings/20261030T2005Z-security-k6-windows-delta-bd7a2bf.md),
  source68631c0..bd7a2bf, dixempreintes comparées et contrôlesdiffcheckexit0.
- [Tests et vrai préflight statique](87-correctifs-integres-et-22-tests-cibles-reussis.md).
- Reprise : disposition de la durée de protection, correction si nécessaire,
  validation ciblée puis relecture du seul fragment concerné. Pas de réauditglobal,
  nouveauGOdejalon ou repriseWSL.
