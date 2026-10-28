# 39 — Fonctions Context Mode : validation et clôture

## 1. Résultat
Implémentation terminée dans le périmètre demandé, sans serveur ni dépendance.
**24 tests Node PASS, 0 FAIL, 0 SKIP**, plus smoke réel de la bibliothèque dans
Context Mode. Pas de nouvelle campagne de sessions pour mesurer un comportement
spontané ou un gain de coût. Rapports précédents :
[préparation37](37-fonctions-context-mode-preparation.md),
[intégration38](38-fonctions-context-mode-implementation.md).

## 2. Changements livrés
### Consignes génériques globales, canon unique
- C:/Users/djden/.agents/context-mode-script-policy.md : chemins localisés/vérifiés,
  données séparées du code, sérialisation des appels imbriqués, syntaxe pour les
  scripts complexes, codes de sortie explicites, diagnostics bornés, préservation
  des résultats voisins et décision de capitalisation sélective.
- Référence ajoutée à C:/Users/djden/.config/opencode/AGENTS.md et
  C:/Users/djden/.codex/AGENTS.md ; nouvelle règle
  C:/Users/djden/.cursor/rules/context-mode-script-policy.mdc.
- Aucun détail AIForAll ajouté au canon générique ; les adapters pointent vers le
  même document. Pas de bibliothèque globale de fonctions ou de nouveau plugin.
  Autres anciennes règles globales non réécrites dans ce lot.

### Skill de fonctions local
`.agents/skills/aiforall-context-functions/` :
- SKILL.md, catalogue.json et examples.md : découverte courte, signatures/limites,
  invocation via JSON.stringify sans échappement manuel et CLI JSON sur stdin.
- scripts/helpers.cjs : readEvidence, classifyCommandResult,
  collectIndependentReads. Lectures préflightées avec lignes et limites explicites ;
  classification de résultat sans exécution de commande ; collecte sans perte
  des voisins. Aucun code métier, outil de recherche ou MCP remplacé.
- scripts/run.cjs : list / describe / invoke ; allowlist fixe, pas d'eval,
  d'import de chemin provenant du catalogue ou de scan de candidates/.
- candidate-template.json et candidates/README.md : code paramétré, provenance,
  dépendances, effets, vérifications et limites. Sauvegarde seulement dans le
  mandat d'écriture ; read-only = proposition sans écriture. Promotion explicite
  après revue/tests/smoke, jamais après un seul succès.
- scripts/helpers.test.cjs et VALIDATION.md : tests et preuves reproductibles.

Petits ajouts à AGENTS.md, au workflow MCP et à fallbacks.md pour rendre le skill
accessible et préserver le contrat local. opencode.jsonc chargeait déjà
.agents/skills : **aucune modification de configuration nécessaire**.

## 3. Vérifications exécutées
- Node Windows v24.11.1 : --check sur les trois fichiers .cjs, exit0.
- `node --test --test-reporter=tap .agents/skills/aiforall-context-functions/scripts/helpers.test.cjs`
  depuis la racine : 24tests,24pass,0fail,0cancelled,0skipped,0todo ;816,2788ms.
  La première version avait également exit0 ; la revue a ajouté une couverture
  et correction du budget total pour les lectures binaires/UTF-8 en échec avant
  le run final24. Pas de boucle de réparation d'infrastructure.
- Couverture : chemins absents/absolus/traversée/drive-relative, espaces/Unicode,
  junction sortante réellement testée, fichier/dossier, mauvais paramètres,
  CRLF/LF/CR/vide, plages hors bornes, binaire/UTF-8 invalide, budgets explicites
  sans ligne coupée ; rg0/1/2, autresprogrammes, timeout/signal/maxBuffer simulés,
  sortie/exception privée non renvoyée, collecte avec rejets et erreurs fulfilled.
- CLI : stdinJSON, erreurs bornées, metadata, module-only, catalogue injecté en
  copie temporaire. Un candidat ne devient pas exécutable ; même une fausse entrée
  validated ne peut étendre l'allowlist.
- Context Mode réel, Bun1.4.2 : require absolu fonctionne depuis le cwd temporaire,
  lecture annotée et ENOENT voisin conservés, collecte complète, rg sur stdin
  réellement classé success/no-match. Smoke final après correction du module : OK,
  y compris appel CLI via JSON stdin.
- Invocation imbriquée via JSON.stringify avec paramètres contenant apostrophes,
  guillemets et saut de ligne : pas d'erreur de syntaxe ; chemin invalide en ENOENT
  sans perte du succès voisin. L'exemple est dans examples.md.
- Découverte du skill annoncée par le harness et chargement via l'outil skill
  réellement réussi. Liens internes vérifiés, JSON catalogue/template valides,
  description frontmatter quotée/valide et sous1024caractères, SKILL sous500lignes.
- Les trois adapters globaux référencent le même document existant ; intégration
  locale présente. gitdiff--check ciblé exit0 ; whitespace des nouveaux fichiers
  vérifié sans anomalie.

Empreintes finales du code exécutable/testé, SHA256 :
- helpers.cjs : a07c8a4e7dd08ab8d43f29c2bbff7db06f79db4f09e10347450d3496e29529e7
- run.cjs : 20aeb2ab2727806e2f7f6dd01394e61bef2f758ee29f3a40bbd17ca0e5124f09
- helpers.test.cjs : 18c233dbd417a79ceadab63ced8476c5b280ac4aa2164f1a636336d4a0187ac0

## 4. Limites et sécurité
Revue par l'auteur, pas review indépendante. Fonctionnement testé dans ce client,
Node24/Bun1.4.2 sous Windows ; pas de sessions Cursor/Codex ou qualification Linux.
Les references globales sont installées, pas une preuve d'obéissance future du LLM.
Aucun gain financier/latence ou parcours déterministe garanti.

Le helper n'est pas une sandbox OS : autorisations du client et confidentialité
restent obligatoires. Les contrôles de chemins portent sur l'état observé, pas
sur une substitution concurrente hostile après préflight. Aucun redactor universel.
maxOutputChars couvre le contenu, pas toute l'enveloppe JSON ; returnedRanges et
truncated imposent d'élargir les preuves coupées avant une conclusion d'absence.
Node18 est le minimum déclaré, non qualifié ici. Timeouts/signaux sont testés par
objets simulés, pas un processus long réel. Les données sources ne sont pas mises
en cache. Les candidates futures doivent fournir leurs propres preuves.

## 5. État final et périmètre
Les tests retirent uniquement leurs fixtures temporaires possédées ; vérification
finale : aucun dossier context-functions-test- restant dans le répertoire approuvé.
Tous les processus de vérification sont terminés. Aucun runtime Docker/Kind/WSL
démarré/arrêté ou ressource Docker créée ; aucun Cargo, build applicatif, purge,
restart MCP, déchargement8B ou changement QMD/Obsidian/Serena.

Les changements préexistants sont conservés ; pas de commit, stash, reset, merge
ou push. Consignes globales horsGit du projet : ne pas les oublier dans une future
sauvegarde/portabilité. Aucun blocage ni demande d'autorisation supplémentaire.
