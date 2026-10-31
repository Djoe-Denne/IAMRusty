# Préparation de la simplification k6 Windows

## État et admission

Décision explicite de l'utilisateur : retirer la sur-ingénierie du lanceur et
exécuter le cas réel `about:blank` avec `6 * 7 === 42`.

La révision3 est désormais approuvée par le start utilisateur, hash
`82a5aefa6a4dcd7deb05121d85b35da21b567585b615661f568ff45d27a22bb6`.
Le lead publie les tâches sous cette révision ; aucune nouvelle approbation n'est
requise à chaque jalon. Les clauses de la révision2 sont remplacées explicitement,
pas réutilisées pour bloquer l'exécution sur les mécanismes rejetés.

## Livrable minimal après approbation

- L'architecte consigne la supersession des seules clauses ACL/verrous/attestation
  du contrat Windows. La proposition `k6-windows-code-custody.md` n'est pas ratifiée
  et n'est pas à implémenter. Pas de nouvelle ADR, enquête ou protection à inventer.
- Un seul auteur reprend les fichiers couplés `run-windows.ps1`,
  `lib/windows.ps1`, `lib/windows-native.cs`, leurs tests Windows et le guide.
  Réutiliser si possible la session de l'auteur existant, sous la nouvelle admission.
  Son premier livrable est déjà intégré en bd7a2bf ; disposition explicite de l'ancienne
  tâche comme livrable source remplacé, sans prétendre qualifier les vrais outils.
- Supprimer les verrous personnalisés des exécutables/DLL et les validations ACL.
  Retirer l'attestation du renderer et les refus conditionnés par son absence.
  Ne pas remplacer ces mécanismes par une autre infrastructure de protection.
- Conserver le manifeste `config/tools.windows.lock.json` : versions exactes,
  provenance et contrôle simple SHA256 avant usage. Ce fichier n'est pas un verrou
  système. Pas de téléchargement ni d'installation implicites.
- Conserver le sandbox natif sans `no-sandbox`, le profil temporaire séparé,
  le contexte neuf, les fermetures normales du scénario, le timeout et le suivi/
  nettoyage raisonnable des seuls processus créés. Pas de terminaison forcée implicite.
- Garder la preuve de version via `browser.version()` dans le même cas déjà ratifié.
  Pas de nouvelle exigence sur Chrome `--version`, les tokens ou les niveaux d'intégrité.
- Supprimer les tests propres aux mécanismes retirés. Garder les contrôles utiles
  de versions, résumé courant non vide, assertions exécutées, timeout et nettoyage.
  Aucun nouveau harness et aucun cycle de revues de risques hypothétiques préalable.

## Vérification et essai réel

Intégrer le delta et effectuer une vérification courte du code effectivement
conservé. Puis lancer le mode navigateur natif avec les chemins déjà préparés,
un répertoire de run neuf et les versions épinglées. Le premier cas ne navigue
que sur `about:blank`, vérifie `6 * 7 === 42`, puis ferme le navigateur.

Reporter le code retour, les assertions réellement exécutées et le nettoyage.
En cas d'échec, conserver des logs privés bornés et exploiter les processus
effectivement observés. Un PASS fonctionnel ne prétend pas certifier le sandbox.
Aucune protection supplémentaire sans défaut reproductible ou exigence explicite.

## Invariants non modifiés

Windows natif uniquement ; WSL suspendu, aucun Docker/Kind, déploiement, tunnel ou
dispatch GitHub. Aucun HTTP applicatif dans ce premier essai, OAuth, seed ou charge.
Versions k6 2.3.0 et Chrome for Testing 156.0.8078.73 ; aucun fallback Chrome149.
Préserver les fichiers étrangers, archives et arbres historiques. Les preuves
Windows ne qualifient pas Linux ; les deux jobs Linux restent suspendus.

## Sources et continuité

Politique préparée : `.opencode/missions/team_mvtz6p47_000q_4wmht0/model-policy.json`.
Décision et état détaillé : rapport93 de cette mission. Ledger canonique :
`C:/Users/djden/source/repos/AIForAll/.cursor/handoffs/20261029-k6-01-socle.md`.
Dernière base vérifiée : `96142df` sur `master`, source Windows intégrée `bd7a2bf`.
Les22tests historiques ne prouvent pas la simplification future ; aucun k6/Chrome
Windows réel n'a encore été exécuté par cette campagne.
