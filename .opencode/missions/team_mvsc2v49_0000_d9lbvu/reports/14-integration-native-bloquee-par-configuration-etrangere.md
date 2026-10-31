# Laboratoire intégré par le mécanisme natif, configuration étrangère préservée

**Résolu.** La branche livrée référait encore une ancienne version du SDK.
Le contrôle natif comparait ce pointeur au canon et signalait un chevauchement
avec la configuration étrangère. Aligner la branche sur le seul SDK déjà publié
et approuvé a suffi : le merge natif a ensuite réussi, sans déplacer ou masquer
la configuration utilisateur et sans modifier le plugin.

Les17 fichiers reçus ont été vérifiés identiques au commit auteur, puis intégrés
au36ffab6. Dix tests d'assemblage du profil ont ensuite réussi dans le canon avec
les nouvelles configurations IAM, hors ligne et sans infrastructure. Cela ne
qualifie pas encore TLS, le réseau ou le véritable navigateur.

## Historique du blocage

La mission doit intégrer le laboratoire livré avant les revues et les tests réels.
Son auteur a terminé17 fichiers et corrigé le dernier oracle source périmé. Le
mécanisme natif d'intégration refuse toutefois de continuer parce que le
sous-module contient une configuration Serena modifiée par l'utilisateur.

Cette configuration est étrangère à la mission : elle est préservée et ne doit
pas être committée, supprimée, cachée ou modifiée pour faire passer le contrôle.
La branche du laboratoire et ses livrables sont également conservés. Aucun code
du laboratoire n'est encore intégré et aucun résultat réseau n'est revendiqué.

Deux essais bornés ont identifié le blocage : après le premier refus, le seul
document QA modifié par le lead a été committé ; le second refus ne cite plus que
le sous-module. Rejouer le merge sans état nouveau ne serait pas utile.

Le coordinateur transfère ce blocage au profil de secours dans la même session,
sans deuxième coordination et sans développement du plugin. La mission
n'autorise que les variantes du modèle Sol : aucun modèle Astra non approuvé
ne sera sélectionné silencieusement. Les auteurs peuvent poursuivre leurs
travaux indépendants autorisés ; leur slot Cargo et les interdictions runtime
restent inchangés.

## Détails de continuité

- Laboratoire livré : HEAD `f5140440c092b4aeb76df8d8c5f8f8dbb2d5ef2d`,17 fichiers, branche propre vérifiée ; auteur shutdown demandé.
- HEAD canon : `5f0a6a0676d19af0990a8e356aa8b774b333fc50`, SDK77672b8 publié/pincé. Seule modification interne SDK : `.serena/project.yml`, étrangère, empreinte préservée lors des pins.
- Premier team_merge pool-8 : refus QA + rustycog. Document QA seul committé5f0a6a0, whitelist/diffcheck.
- Second team_merge pool-8 : refus rustycog seul. Pas de stash/reset/force ni appel Gitmerge manuel.
- Source et preuves auteur : rapport final9c114db puis microfixf514044 ; revue du diff deploy/images déjà faite par le lead, autres fichiers à inspecter/intégrer.
- Tentative suivante de patch interrompue par transport WebSocket1000 avant tout contenu ; après reprise, gitstatus confirme aucun delta nouveau et mêmeHEAD. Ce n'est pas une panne fonctionnelle du produit.
- Aucune opération Docker/Kind/WSL/IT/browser lancée. App pool-3 possède le bail Cargo final ; son dernier retour indique checksHive et6unités nouvelles verts, monolithe4E0609 suspectés métadonnées partagées, une expérience bornée en cours. Ne pas démarrer un autreCargo.

## Résolution vérifiée

- Diagnostic lecture seule du garde natif : comparaison HEAD→branche, ancien gitlinkd60 contre776 canonique ; aucun patch du plugin ou changement de permission.
- Seul index du worktree laboratoire arrêté : sélectionner le gitlink publié776 depuis le commit canon2b0ea47, commit additionnel9bcc100, historique antérieur conservé. Aucun rebase, reset, stash, force ou fichier utilisateur touché.
- team_merge natif réussi ensuite ; cleanup worktree pending, branche conservée. Aucun merge Git manuel.
- Empreinte SHA256 de la configuration Serena étrangère identique avant/après. Hash des17 fichiers intégrés exactement égal à la livraisonf514044.
- Commit canon36ffab6, whitelist17/diffcheck, hook persistant.githooks inchangé. Tâche laboratoire acceptée après intégration.
- Run canon : python unittest du profil,10 tests/exit0,1.975s, rendu en mémoire avec configIAM intégrée. Aucun Cargo/runtime lancé par ce contrôle.
