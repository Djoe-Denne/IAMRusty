# 25 — Ajustements des instructions agents : préparation

## Mandat et périmètre
L'utilisateur autorise les modifications agents proposées après l'audit et le
diagnostic Serena. Travail direct, sans nouvel agent : contrat `AGENTS.md`, skill
local `aiforall-mcp-workflow` et sa référence de repli. Les profils renvoient déjà
à ce canon ; ne pas y recopier les nouvelles procédures.

## Décisions d'implémentation
- Préserver les résultats d'appels de lecture indépendants malgré une erreur
  dans un autre appel, attendre leur terminaison et examiner aussi les erreurs
  renvoyées dans des réponses techniquement résolues.
- Cibler les corps/consommateurs utiles avant les références très transversales ;
  vérifier les chemins inconnus et réutiliser catalogue, preuves et résultats
  encore valides. Aucun quota d'appels ni usage systématique des deux MCP.
- Garder le droit à une lecture comparative groupée lorsque plus adaptée : ne
  pas assimiler automatiquement volume élevé ou choix de fichiers à un défaut.
- Distinguer attente en file, calcul serveur et coût initial. Ne pas présenter
  le déplacement de l'attente ou un préchauffage comme un gain gratuit.
- Pas de code MCP, nouveau résumeur LLM, préchauffage massif, cache reset,
  changement de modèle/configuration globale ou nouvelle campagne métier.

## Vérification prévue
Contrôles ciblés des références et métadonnées du skill, diff/espacement ; exemple
d'orchestration vérifié par cas synthétiques sans API/MCP/runtime réel. Le gain
comportemental et la vitesse ne seront pas déclarés prouvés par ce test documentaire.
Preuves réutilisées : rapports22–24, sans rejouer la session de l'utilisateur.
Aucune décision bloquante à demander ; le keepalive 8B reste inchangé.
