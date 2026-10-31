# Recentrer le lanceur et exécuter le premier cas réel

## Objectif et décision utilisateur

Faire fonctionner une campagne k6 reproductible, en commençant par Chrome sur
`about:blank` et la vérification `6 * 7 === 42`. L'utilisateur a explicitement
demandé de supprimer les verrouillages personnalisés des exécutables/DLL, les
contrôles d'ACL et l'attestation du sandbox. Aucun incident réel sur ces fichiers
n'a été observé. La proposition de conservation persistante n'est donc pas retenue.

Conserver les versions épinglées et leurs empreintes, le sandbox natif Chromium,
un profil temporaire séparé, un timeout et un nettoyage raisonnable des seuls
processus créés. Un résultat fonctionnel ne sera pas présenté comme une certification
du sandbox. Aucune nouvelle protection sans défaut reproductible ou exigence explicite.

## État vérifié et changement depuis l'explication précédente

Le code actuel impose encore les anciens contrôles et le refus `SANDBOX_UNPROVEN`.
Il n'a pas été exécuté contre les vrais outils Windows. Les22tests précédemment
réussis restent des preuves de leur ancienne surface, pas du code simplifié futur.

Le projet de politique a été corrigé pour retirer ces exigences, garder la QA
proportionnée et prévoir le cas réel dès la simplification minimale. Le catalogue
OpenAI, l'autonomie long run et les limites Windows/WSL/Linux restent inchangés.
L'ancienne commande Chrome `--version` n'est plus un prérequis : la version sera
vérifiée par l'API déjà ratifiée dans le même cas navigateur.

## Mise en œuvre préparée

1. Faire consigner par l'architecte la supersession des clauses ACL/verrous et
   attestation, sans nouvelle protection ni nouvelle enquête sur les tokens.
2. Reprendre le même auteur pour supprimer ces mécanismes et leurs tests dédiés.
   Garder le contrôle simple des versions/SHA256, les profils temporaires séparés,
   les assertions du scénario, le timeout et le suivi/nettoyage existant utile.
3. Intégrer le delta, effectuer les contrôles courts adaptés, puis lancer le cas
   réel. Ne pas attendre un nouveau cycle de revues hypothétiques pour le lancer.
4. S'il échoue, exploiter les logs privés bornés, le code retour et les processus
   effectivement observés ; ne pas inventer de défaut ni de protection supplémentaire.

## Passage d'approbation natif

La révision approuvée contient encore une exigence explicite d'attestation du
sandbox. La changer modifie le contrat QA enregistré. Le mécanisme Ensemble impose
donc une présentation `config`, puis un `start` de l'utilisateur pour cette révision.
Ce passage ne demande pas de reconsidérer la décision technique déjà donnée ; il
remplace les anciennes exigences qui bloqueraient autrement l'auteur.

Recommandation : afficher et approuver le projet corrigé avec
`/ensemble-mission config`, puis `/ensemble-mission start`. Conserver l'ancienne
politique laisserait les contrôles rejetés obligatoires. Aucun autre GO de jalon
n'est demandé. Aucun nouveau code ou test Windows n'est exécuté avant cette approbation.

## Preuves et limites techniques

- HEAD observé : `96142df91f59df8d5e4f8a2d423c1b9382b9e96e`, branche `master`.
- Points connus : `ops/tests/k6/run-windows.ps1:35–39,52,72–79,94` ;
  `docs/contracts/k6-windows-functional.md:48–51,109–114`.
- Politique corrigée : `.opencode/missions/team_mvtz6p47_000q_4wmht0/model-policy.json`.
- JSON de cette politique relu et valide :4entrées de modèles OpenAI inchangées,
  QAproportionate,6priorités d'autonomie,10commandes et18critères. `git diff --check`
  réellement exécuté :exit0, stdout/stderr vides. Aucun test Windows lancé.
- Inventaires Git branches/worktrees/sous-module relus en lecture seule. Les arbres
  historiques, le sous-module modifié et les fichiers d'autres missions sont préservés.
- Une première commande d'inventaire a échoué sur une expression JavaScript mal
  échappée ; appel corrigé puis réussi. Pas de panne MCP ni exécution d'outil déduite.
- Aucun k6, Chrome, GET, WSL, Docker ou Kind lancé pendant ce recentrage. Le premier
  essai réel demandé reste à exécuter, et aucun PASS n'est annoncé.

## Reprise automatique du 2026-10-31

Le message persistant reçu est uniquement une notification de réveil du superviseur.
Le statut natif a été vérifié une fois : la politique modifiée reste non approuvée,
avec demande explicite de `config`, puis `start`. Aucun auteur n'a été relancé et
aucun outil réel exécuté. Cette attente n'est pas fondée sur les risques rejetés par
l'utilisateur, mais sur ce passage natif obligatoire.

La préparation utile restante a été persistée dans
`.cursor/review-briefings/20261031-k6-windows-simplification.md` : périmètre de
suppression, protections ordinaires conservées et essai réel prioritaire. Après
approbation, pas de nouvelle enquête de sécurité ni de GO intermédiaire à demander.
