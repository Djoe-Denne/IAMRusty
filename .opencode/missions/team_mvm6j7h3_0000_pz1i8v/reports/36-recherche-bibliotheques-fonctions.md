# 36 — Bibliothèque de fonctions pour Context Mode : solutions existantes

## 1. Objectif et résultat
Recherche Internet demandée par l'utilisateur, effectuée directement, sans agent
supplémentaire. Distinguer une bibliothèque de code réutilisable d'un cache de
résultats, et une solution documentée d'une intégration Windows validée.

**Résultat : les mécanismes existent ; pas besoin de développer un framework.**
Il reste à sélectionner/écrire les fonctions utiles au dépôt et vérifier leur
invocation depuis le runtime Context Mode installé. Aucune installation ou
modification de guidage, de configuration ou de code exécutable effectuée.

Continuité : [deux essais observés](35-autonome-medium-2-comparaison.md).
Les erreurs observées justifient d'étudier la réutilisation ; elles ne prouvent
pas à elles seules le retour sur investissement d'une plateforme supplémentaire.

## 2. Sources primaires consultées le 2026-10-24
Recherche web puis lecture/indexation des documents via Context Mode, avec
extraction des passages utiles. Les affirmations ci-dessous sont documentaires,
pas des résultats de benchmark ou d'audit de sécurité.

### Agent Skills — option légère recommandée
- Spécification : <https://agentskills.io/specification>.
- Un dossier SKILL.md peut embarquer scripts/, references/ et assets/.
  scripts/ contient du code exécutable, avec dépendances explicites, messages
  d'erreur et prise en charge des cas limites. Les langages dépendent du client.
- Le skill sert à découvrir et expliquer l'usage ; le script ou module conservé
  porte l'implémentation déterministe. Le standard n'impose pas un outil universel
  de recherche/exécution ni une validation automatique des fonctions.
- Le mécanisme Skills existe déjà dans cet environnement. Proposer des scripts
  versionnés est un ajout local, pas une modification nécessaire du MCP.

### Pattern explicitement décrit par Anthropic
- <https://www.anthropic.com/research/code-execution-with-mcp>.
- Section « State persistence and skills » : sauvegarder une fonction dans un
  fichier, la réimporter plus tard, et ajouter SKILL.md pour la découvrir.
- Section « Progressive disclosure » : découverte par fichiers ou search_tools.
- C'est une architecture de référence, pas un paquet prêt à installer pour
  Context Mode. Les gains chiffrés de l'article ne sont pas transposables ici.

### ToolRegistry — registre de fonctions existant
- <https://github.com/Oaklight/ToolRegistry>.
- Bibliothèque Python : enregistrement, génération de schémas, exécution.
  Écosystème distinct : toolregistry-server expose via OpenAPI/MCP ;
  toolregistry-hub fournit des outils préexistants, notamment opérations fichiers.
- Python >= 3.10 selon le README consulté. Ce n'est pas un système autonome
  garantissant la correction, la revue ou la sécurité du code généré.
- Pertinent si un véritable catalogue multi-outils est requis ; apporte une
  dépendance et un raccordement à évaluer. Pas testé sous Windows dans cette mission.
- Première URL raw sur main : HTTP 404 ; page canonique GitHub ensuite lue,
  branche master observée. Pas assimilé à une indisponibilité de Context Mode.

### FastMCP — exposer des fonctions comme outils typés
- <https://gofastmcp.com/servers/tools>.
- Décorateur @mcp.tool ; schéma dérivé de la signature/docstring, validation
  d'entrée et exécution des fonctions. Gestion des erreurs MCP documentée.
- Un framework de publication, pas un stock de fonctions déjà adaptées au dépôt.
  Nécessite de développer les helpers et d'intégrer un serveur supplémentaire.
- Alternative à une bibliothèque locale si l'on veut des appels structurés qui
  évitent la génération de code d'invocation, sous réserve du client et de son
  routage. Ne pas supposer le partage automatique du contexte/index Context Mode.

### Windmill — plateforme de scripts existante
- <https://www.windmill.dev/docs/core_concepts/mcp>.
- MCP documenté : listScripts, getScriptByPath, runScriptByPath,
  gestion de scripts/versions et flows ; accès soumis aux autorisations workspace.
- Répond réellement à « catalogue de scripts interrogeable et exécutable ».
- Apporte une plateforme et des contraintes d'exploitation/intégration :
  disproportionné pour quelques helpers de lecture locale, selon notre évaluation.
  Pas d'installation, d'accès cloud ou de compatibilité au filesystem Windows
  local validés ici ; aucune recommandation de transmettre le dépôt à un service.

## 3. Situation de Context Mode
- README amont : <https://github.com/mksglu/context-mode> ; lecture directe de
  <https://raw.githubusercontent.com/mksglu/context-mode/main/README.md>.
- Chaque ctx_execute est décrit comme un sous-processus séparé : ne pas compter
  sur une fonction définie en mémoire lors d'un précédent appel. Un fichier/module
  réutilisable est différent d'un état mémoire partagé.
- La documentation examinée ne présente pas un registre natif de fonctions
  validées avec découverte/promotion automatiques. Ce constat documentaire ne
  prouve pas l'absence de toute fonctionnalité dans chaque version ou extension.
- L'accès/import de modules locaux depuis la version installée, les chemins
  Windows, dépendances et permissions restent à vérifier avant implémentation.
  Une invocation courte limite les échappements, sans les abolir entièrement.
- Un serveur MCP séparé n'hérite pas automatiquement du traitement/index Context
  Mode : planifier comment garder les gros résultats hors conversation.

## 4. Recommandation et arbitrage
Commencer, si l'utilisateur autorise une implémentation, par un Skill avec quelques
scripts/modules versionnés et vérifiés, et un contrat d'appel court. Préférer des
post-traitements répétitifs réellement observés à une duplication de grep natif,
Serena, Rust MCP ou des primitives déjà disponibles.

Si le catalogue devient important, comparer ToolRegistry et FastMCP. Ne pas
introduire Windmill pour le seul besoin actuel. Ne pas développer un nouveau
registre avant d'avoir démontré que le simple packaging Skills est insuffisant.

Les bibliothèques existantes résolvent emballage/découverte/exposition, pas
automatiquement la qualité des fonctions. Pas de promotion aveugle d'un script
généré dans une bibliothèque de confiance ; conserver revue, tests ciblés,
versions, erreurs explicites et restrictions de périmètre.

## 5. Limites et clôture
Aucun paquet installé, aucune fonction implémentée ou exécutée, aucun essai
OpenCode supplémentaire. Pas de gain de coût/latence mesuré ni de garantie de
compatibilité. Seul ce rapport est ajouté au dépôt pour cette recherche.
Aucun Cargo/Docker/Kind/WSL, cache vidé, modèle déchargé ou runtime démarré.
Pas de décision bloquante demandée : restitution des options, sans élargissement
automatique vers l'implémentation.
