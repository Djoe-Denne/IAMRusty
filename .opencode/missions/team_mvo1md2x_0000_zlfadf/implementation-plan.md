# Outbox Telegraph — plan courant après recherche et nettoyage

## Révision opératoire du 2026-10-27

### Livraison enregistrée — état final141

**Suite142 :** demande utilisateur commit/push gitlink inclus exécutée. Le commit
application39024b2 est désormais disponible sur origin/mission-telegraph-outbox-integration,
SHA distant vérifié ; SDKd60 publié et gitlink committé confirmés, arbre propre.
La mention «commitlocalnonpoussé» de141 est historique et levée. Pas de merge
dans la branche principale, nouvelle QA ou modification des autres réserves.

Suites134 et livraison SDK/pin achevées : verticale finale1PASS, revue sans
blocage, SDKd60 publié sur branche dédiée, gitlink160000d60 committé dans
AI39024b2b445ae9e35f9ad148ea754995f8baab18. Arbres AI/SDK propres,30 tâches
complétées,aucuneactive. Hookfmtall normal0, aucunefixturepossédéeactive.
Rapport141 décrit preuves et réserves : Clippy strict antérieur non vert,
Sonar/CI distante non exécutés, applicationcommitlocalnonpoussé ; pasPASSglobal
ni promotionADR. Sections suivantes = historique de la séquence, pas relance.

### Suite de mission reprise et autorisée en134

Utilisateur : «reprends», puis «Adapter la preuve complète» après rapport133.
La tranche132 reste acquise ; ne pas la relancer.

1. Adapter le seul test vertical avec échéance fixe liée à l'éligibilité persistée,
   <=55s reconstruction incluse et transport sortant couvert ; IAM25s et tous les
   oracles métier/replay/droits inchangés. QA canonique
   `.cursor/qa-plans/20261027-vertical-post-expiry134.md` et contrat source dans
   TI/.tmp/vertical-final134-inputs. Préparation/compilation uniquement, pas de run.
2. Intégrer le delta puis revue indépendante ciblée du timing et de la possession
   des tâches ; fermer les défauts bloquants sans pipeline global.
3. Préflight puis UNE validation verticale ciblée, ressources et cleanup tracés.
   Pas de rerun automatique. Publication non couverte par cette exécution.
4. En parallèle de1 : inventaire RO de disponibilité SDK, preuves existantes et
   gates de livraison. Aucune écriture/push/pin. Après3 : bilan puis appliquer
   seulement les suites de livraison réellement autorisées et satisfaites.

Auteur test : pool23/test-engineer, session déjà admise pour134. Continuitépool14
demandée mais non réalisable avec affectation native exposée (STOPrespecté puis
spawnexistingname refusé) ; repli explicite lead depuis snapshots vérifiés,
contexte neuf assumé et ancien auteur/instrumentation préservés, pas override modèle.
Tasktest task_mvr1g9bq_0002_ozw23g ; rechercheSDK task_mvqyq8pr_0000_eaa72e.
Consommateurrevuenonpubliéavantlivrableintégré. SlotCargo pas encore ouvert.

### Tranche ciblée antérieure — historique132

**État après exécution :** étapes1–4 ci-dessous achevées pour la tranche ciblée.
Run `post-expiry122-6412a9c40ae64bf4` :1 PASS/0 échec, preuve après expiration
acquise sur le composant, cleanup recoupé0 possédé actif. Rapport132 fait foi
pour la portée et les réserves. Aucun run suivant, publication ou pin déclenché.

Cette section gouverne la suite ; le document antérieur ci-dessous reste une
archive, pas une instruction de préparation ou de réapprobation de la mission.
La politique native v2 déjà approuvée reste inchangée, QA proportionate.
Le PM a été sollicité puis repris dans sa même session, mais aucun résultat
natif récupérable n'a été livré. Le lead assume explicitement cette révision ;
elle n'est pas présentée comme un livrable PM accepté.

### Travaux conservés et décisions closes

- Préserver AC1–AC7, transaction writer commune, transport NoOp suspendu,
  propriétaire HTTP et corrections port 0 / CONNECT, oracle firstFAILED106.
- Conserver les deux témoins composant : le témoin avant expiration vérifie
  aussi l'ancien propriétaire et son verrou inchangés ; son équivalence avec
  le témoin complet n'est pas démontrée. Leur taille seule ne justifie aucun retrait.
- Le dispatcher SDK est inchangé depuis le pin129 ; aucun correctif contraire
  à retirer n'est établi. Garder lease30s, poll2s, backoff5s, max20.
- Retirer du chemin actif les enquêtes exploratoires répétées, le drain
  coopératif facultatif, le déverrouillage anticipé et le raccourcissement de lease.
  Ne supprimer ni tâches historiques complétées, ni sources et résultats archivés.

### Graphe de la suite autorisée

1. **Audit et nettoyage ciblé — lead.** Audit technique fait : aucun changement
   de production contraire trouvé. Nettoyer la photographie documentaire et les
   consignes obsolètes, sans changer contrat normatif, test revu ou lanceur validé.
   Deux documents possédés par le lead après information de l'auteur documentaire.
2. **Vérifier le nettoyage — lead, après 1.** Examiner le diff exact ; vérifier
   qu'aucune source Rust, aucun SDK, oracle, budget ou pin n'a changé ; contrôler
   les liens et `git diff --check`. Si les tests restent identiques aux snapshots
   compilés et revus, réutiliser ces preuves : pas de compilation redondante.
3. **Rattacher la preuve composant après expiration — après 2.** Reprendre le
   composant revu avec identifiant de campagne valide et préflight frais. Une seule
   exécution ciblée, pas le redémarrage de l'ancienne question127 ni une campagne
   libre. Deadline fixe t0+R+25s, R mesuré une fois et <=30s, maximum55s depuis
   reconstruction incluse ; cleanup6s/phase61s, IAM25s inchangés. Vérifier avant et
   après l'éligibilité persistée, identité/payload, nouveau claim, vrai transport
   SQS, état published et comptes métier inchangés. Respecter slot Cargo Windows
   unique, jobs24, fixtures existantes, bail et registre contemporains.
   L'exécution ne commence pas avant nettoyage vérifié et contrat runtime borné.
4. **Consigner résultat et limites — lead, après 3.** Un échec d'initialisation
   reste distinct d'un défaut fonctionnel. Pas de rerun automatique. Le PASS du
   composant ne ferme ni verticale, ni droits/replay, ni toute l'application.

La demande utilisateur de reconnecter l'exécution après nettoyage définit cet
ordre, sans autoriser d'extension. La verticale finale, la publication SDK et le
pin final restent des suites séparées bloquées, non lancées par cette révision.
Aucune nouvelle architecture ni nouveau harness. Les revues existantes restent
valables pour les sources inchangées ; nouveau risque réel = vérification ciblée,
pas pipeline imposé. Toute contradiction de permission runtime est remontée
avant exécution ; ne pas déduire un GO d'une autorisation historique consommée.

Sources : rapports129/130, ledger `.cursor/handoffs/20261025-telegraph-notification-outbox.md`,
contrat `docs/contracts/telegraph-notification-outbox.md` et sources TI auditées.

---

# Archive — amendement de continuité retiré

Date : 2026-10-25. Mission : `team_mvo1md2x_0000_zlfadf`.
État : préparation uniquement ; aucune autorisation de démarrage.

## Décision utilisateur applicable

L'utilisateur a retiré la recommandation de fork T1 vers T2 et demande de suivre
scrupuleusement le plan natif importé. Ce retrait est définitif pour le mandat
courant : aucune correction de continuité n'est différée à une prochaine étape.

Le présent document n'est plus un amendement opératoire. Les descriptions,
dépendances, critères et briefings du plan importé font autorité.

- T1 : architecture fresh, contrat écrit examiné et intégré avant T2.
- T2 : implémentation fresh, à partir du contrat et des conclusions écrites
  de T1 ; aucun fork T1 vers T2 prévu par la recommandation retirée.
- T3 : reprendre de préférence le propriétaire et la session de T2 comme le
  prévoit déjà la tâche importée ; si impossible, choix explicite du superviseur
  d'un contexte fresh avec les livrables intégrés.
- T4 : reviewer fresh indépendant, lecture seule.
- T5 : documentation fresh en worktree isolé, parallélisable avec T4 après T3.

La suite ci-dessous est historique et non applicable ; elle conserve seulement
la trace de la proposition retirée. Ne pas la joindre comme instruction active
aux briefings ni tenter de la réappliquer ultérieurement.

## Historique de la proposition retirée

Ce document complète les cinq tâches importées depuis la préparation PM
`ses_e8c105ef6ffeMYtpy2DqPiBKdh`. Il remplace uniquement leurs indications de
continuité contradictoires ci-dessous ; périmètres, dépendances, propriétaires
de fichiers et critères AC1–AC7 restent ceux des tâches importées.

Le tableau natif n'a pas été réécrit : l'outil exposé permet l'ajout de tâches
et les décisions d'exécution, pas l'édition de leur description. Le lead doit
joindre ce document aux briefings et appliquer ces choix à l'admission, sans
dupliquer les tâches ni contourner la politique modèles.

## T1 — Contrat architectural

- ID : `task_mvo2evlb_0003_oigv2n` ; profil `architecte`.
- Contexte fresh et worktree isolé, comme prévu dans la préparation.
- Livrer le contrat versionné, les interfaces, les invariants, les arbitrages,
  les alternatives rejetées et les oracles AC1–AC7.
- Le lead examine et intègre le contrat avant T2. Toute décision structurelle
  encore ouverte reste explicitement ouverte ; pas d'Accepted autonome.
- Conserver la session native T1 comme source de fork pour T2. Son identifiant
  réel sera enregistré après admission ; le PM n'est pas cette session.

## T2 — Atomicité métier + événement durable

- ID : `task_mvo2evlb_0004_04vtof` ; profil `hard-implementer`.
- Dépend de T1 validé et intégré.
- **Correction : fork natif de la session T1, au lieu du contexte fresh.**
- Nouveau propriétaire d'implémentation, worktree isolé ; pas de handoff de
  l'architecte vers le code dans son répertoire partagé.
- Avant admission, vérifier source native disponible, capacité de fork,
  états Git propres et HEAD compatibles entre source et destination.
  Un fork ne copie pas les fichiers non intégrés ni les commits source-only.
- Le briefing contient le chemin absolu du contrat, le HEAD intégré, les
  décisions validées primant sur les hypothèses historiques, le périmètre
  d'écriture, les contrôles attendus et les conditions d'arrêt. Il ne recopie
  pas toute l'histoire déjà héritée.
- Les nouveaux changements de contrat sont transmis explicitement au membre :
  le fork n'est pas une synchronisation live.
- Si le fork échoue ou entre en conflit avec la politique modèle, suspendre
  l'admission concernée et signaler le blocage. Aucun repli fresh, reset,
  stash, commit utilisateur ou changement de modèle silencieux.

## T3 — Supervision et preuve jusqu'au droit HTTP

- ID : `task_mvo2evlb_0005_lu32q6` ; profil `hard-implementer`.
- Dépend de l'intégration vérifiée de T2.
- **Continuer avec le même propriétaire et la même session native que T2**,
  sans nouveau spawn ni fork de synthèse.
- Préserver le modèle admis ; toute nouvelle admission respecte sa politique.
- Transférer explicitement la propriété de setup, fixtures et Cargo.lock.
  Réconcilier la version intégrée avant de continuer, sans écraser de travail.
- Le profil restant identique, aucun handoff de spécialité n'est nécessaire.
  Si un changement devient utile, il conserve session, répertoire et modèle ;
  il n'ajoute ni isolation ni leadership.
- Une reprise native impossible impose une décision explicite du lead,
  rapportée avant toute nouvelle session ; pas de repli silencieux.

## T4 — Revue indépendante

- ID : `task_mvo2evlb_0006_52988o` ; profil `correctness-reviewer`.
- Après T3 intégré : contexte **fresh**, lecture seule, `worktree: false`.
- Aucun fork de T1/T2/T3 ou du contexte de développement du lead.
- Entrées : contrat validé, snapshot figé, diff et preuves observables.
  Ne pas transmettre les conclusions rassurantes du développeur ou un verdict
  prérempli. Aucun contrôle supplémentaire sans nécessité concrète.

## T5 — Documentation factuelle

- ID : `task_mvo2evlb_0007_pk88da` ; profil `implementer`.
- Après T3 intégré : contexte fresh et worktree documentaire isolé.
- Entrées : contrat, code intégré et résultats vérifiés ; pas de dépendance
  au verdict de T4 et pas de clôture annoncée avant le lead.

## Dépendances et clôture

`T1 -> T2 -> T3 -> (T4 || T5)`.

T1/T2/T3 ne sont pas parallélisés. T4 lit le code figé pendant que T5 écrit
uniquement les documents. Le lead attend les deux, renvoie les défauts
bloquants au propriétaire T2/T3, puis réconcilie les faits documentaires affectés.

La référence durable reste le contrat versionné et les preuves ; l'historique
natif ne les remplace pas. Le lead persiste les HEAD, sessions réelles,
décisions et tentatives dans le ledger commun, sans écritures concurrentes.

## Garde-fous inchangés

QA proportionnée ; pas de classifieur ni pipeline QA supplémentaire.
Un seul Cargo Windows, 24 jobs configurés sans override. Fixtures existantes,
inventaire et registre Docker contemporains, arrêt des ressources possédées.
Pas de Kind, nouveau backend, modification du framework ou de la configuration
globale. La correction de modèles Luna/Sol reste à enregistrer et à présenter
via config avant tout start ; ce document n'approuve aucune politique.
