# 145. Les deux fonctions sont incluses dans la bibliothèque utilisable

## Résultat

Sur demande explicite de l'utilisateur après la préparation, promotion locale
terminée : le catalogue comporte désormais cinq fonctions validées au lieu de
trois. Les deux nouvelles fonctions s'importent depuis le module public
`scripts/helpers.cjs` ; aucun code du dossier candidates n'est chargé.

- `compareOwnedContainerInventories(inventory, options)` : comparaison pure de
  captures. Disponible aussi via la CLI `compare-owned-container-inventories`,
  stdin `{inventory, options?}`.
- `decodeEvidenceText(bytes, options)` : décodage strict UTF-8/UTF-16. Module
  seulement, comme les autres fonctions qui demandent une entrée non JSON ; pas
  de conversion implicite d'un tableau ou chaîne vers Buffer.

Skill et exemples actualisés pour permettre leur découverte. Les anciens dossiers
de candidates conservent la provenance et pointent vers les modules/tests promus,
sans duplication de code exécutable.

## Vérification

Revue directe du diff : modules déplacés sans changement de logique, exports,
catalogue et allowlist fixes cohérents, quarantaine des candidates conservée.
Tests exercés par l'API publique, deux tests supplémentaires sur l'intégration CLI.

- Sept vérifications de syntaxe Node : PASS.
- Trois suites sous Node v24.11.1, reporter TAP explicite : **49 PASS, 0 FAIL,
  0 SKIP, 0 CANCELLED**, exit 0 et stderr vide.
- Context Mode Bun 1.4.2 : dix assertions PASS sur les exports réels, scénarios
  positifs/négatifs, découverte des cinq fonctions et appel CLI comparé au module.
- Métadonnées de promotion et chemins vérifiés ; absence d'import runtime de
  candidates vérifiée. Preuves détaillées dans le VALIDATION.md de la bibliothèque.

Pas de revue indépendante ou de qualification Linux/macOS/Node18 revendiquée.
Pas de ressource Docker, Cargo, Kind, WSL ou dépendance supplémentaire.

## Limites préservées

La comparaison ne prouve pas elle-même propriété, baux ou complétude des captures ;
elle n'autorise aucune suppression. Un code CLI 0 peut transporter un résultat
inconclusive. Le décodeur n'expurge pas les secrets du texte retourné et n'interprète
pas les résultats de tests. Les limites de données restent documentées au catalogue.

## Fichiers et livraison

Racine : `.agents/skills/aiforall-context-functions/` dans l'arbre principal.
Modules ajoutés sous scripts : compare-inventories.cjs et decode-evidence.cjs,
avec leurs tests ; helpers, runner, catalogue, exemples, skill et validation mis à
jour. Les dossiers de provenance portent désormais `status: promoted`.

Arbre principal HEAD observé : 53c48154f577acab70f9ea85b1de7e3dbb01cc98.
Les deux skills étaient déjà non suivis avant ce travail ; aucun stage, commit
ou push effectué. Le worktree TI ne contient pas la bibliothèque : aucune copie
divergente n'y a été créée.
