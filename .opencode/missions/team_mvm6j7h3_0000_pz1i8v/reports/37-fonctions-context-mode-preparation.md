# 37 — Fonctions Context Mode : préparation de l'implémentation

## Mandat et stade
L'utilisateur autorise l'implémentation de l'ensemble des recommandations :
chemins vérifiés, scripts robustes, réutilisation, découverte et capitalisation.
Travail direct, aucun membre supplémentaire, aucun serveur ou paquet à installer.

## État vérifié
- HEAD : 93e376c5530a439aaafe61a64d3e881865c1acf1 ; 112 entrées Git modifiées
  préexistantes. AGENTS.md, opencode.jsonc et le skill MCP sont déjà modifiés ;
  préserver ces changements, pas de commit/stash/reset.
- opencode.jsonc charge déjà .agents/skills ; aucun changement de configuration
  nécessaire pour le nouveau skill local.
- Serena initialisé, racine AIForAll vérifiée, pas de changement symbolique Rust.
  Nouveaux scripts et edits docs : patch ciblé, pas d'exploration Rust requise.
- Consignes globales identifiées : OpenCode AGENTS.md, Codex AGENTS.md et règles
  Cursor. Les règles Cursor anciennes sur le déploiement MCP ne sont pas corrigées
  dans ce lot. Le contrat local garde sa priorité pour AIForAll.
- Sources de conception : rapport36, skill MCP/fallbacks actuels, documentation
  officielle V2 skills/instructions consultée. Pas de nouveau framework nécessaire.

## Périmètre retenu
1. Politique générique partagée hors dépôt, référencée depuis les trois clients :
   localisation avant batch, code/données séparés, erreurs bornées, réessai ciblé,
   conservation des résultats, choix libre adapté au coût et capitalisation.
2. Skill local aiforall-context-functions avec catalogue court, module CommonJS
   sans dépendance et CLI à opérations explicites. Lecture comparative annotée,
   classification des sorties de commandes, collecte indépendante et fins de ligne.
   Pas de duplication de la recherche sémantique ou de grep natif.
3. Candidats distincts du catalogue validé : sauvegarde locale autorisée dans ce
   périmètre, jamais promotion/exécution automatique après un simple succès.
4. Références locales depuis AGENTS et le workflow ; pas de modification MCP,
   index, mémoire, modèle, serveur ni code métier.

## Validation prévue
Tests Node ciblés : chemins avec espaces/Unicode, chemin absent/hors racine,
liens sortants si création autorisée, erreurs et succès partiels, CRLF/LF,
troncature explicite, rg0/1/2, exécutable absent, timeout/signal, paramètres invalides,
catalogue et exclusion des candidats. Smoke réel du chargement depuis Context Mode.
Pas de campagne de sessions, de Cargo, d'IT, de Docker/Kind/WSL ou de benchmark.

## Inconnues et recommandation
Le chargement du module local depuis le sandbox Windows reste à prouver. Tester
le chemin absolu via require ; ne pas réinstaller Context Mode si ce test échoue.
La bibliothèque réduit la génération de plomberie sans garantir un agent
déterministe, un confinement OS ou un gain financier. Continuer dans ce périmètre.
