# Rapport 93 — diagnostic Ensemble et repli d'intégration borné

## Cause vérifiée par emergency-thinker

Session `ses_e86007390ffejlnQ8Goj03H3xy`, lectureSQLite mode=ro/snapshotmémoire et codeinstallé, aucune mutation/Cargo/runtime. Membrepool12 shutdown/preserved avec **merge_applied=1**. Bundleplugin `C:/Users/djden/.config/opencode/local-plugins/ensemble/20261025-pm-review-0f9902409237/index.js`, SHA2560f99024092376b72e85da143d2bd1ac491ad3cfc6dc55a2dec50bbe791c9c17e.

Lignes16721–16722 : marqueur fait retourner directement finishMerge avant rafraîchissementref/comptage/fusion.16780–16785 : cette voie essaie de retirer le worktree ; échec annoncé «Merged…cleanup pending…retry». Force-message15830–15844 réveille la session sans réarmer ce marqueur ; entréesV2/legacy même mécanisme17243–17251/39808. Les corrections de refs ne pouvaient donc pas être consommées.

**Interdit de réessayer team_merge sur ce membre pour ce suivi** : risque de retrait du worktree sans intégration. PasDBedit, réparationplugin, purge/cleanup ou contournement global proposé. Ce défaut de lifecycle/métadonnées n'est pas un défautducodeHTTP.

## État et décision lead

AIleaddeac598/SDK5eaa inchangés, indexpin129/M SDK seulement. Brancheauteur etpreserved d36, anciencheckpointc956 conservé. d36parentexactdeac, treec05b27, unique fichierruntime+59 ; blobd36=auteur=archivesourceSHA3e21c3017758917b4c4a8a0f743c16b24c1b39acf5d125265f59f52c92197be9. Préflight `git apply --check` archivé effectuéRO par specialist exit0, sans application.

Lead décide dans le mandat d'intégration déjà approuvé : **appliquer par outilpatchdédié seulement le delta tests archivé**, SHApatch340b78eed3b5874c80ad512bb2e7311bdaf2c5b087dd302076da7ac89027c069. Puis vérifier hashsource/arbre exacts et commitnormal courtlead. Pasgitmerge/cherrypickmanuel, reset/stash, gitlink prématuré, DB/harnessmodifié ou oracle réécrit. Cette exception est explicite car l'outilnatifaétéemployéetsondéfautprouvé, pas un repli silencieux.

SDKobserverd60 : intégrationdistincte parbundlehashvérifié après contrôleSDK5eaapropre, paspublication/pinfinal. Réutiliser preuvesC3candidatevert/base3333rouge etC2rouge, ne pas rejouer pour la seule intégration. C2fixproduction reste bloqué avant témoinintégré ; revuesfinales/ITfixture/pin restent gates. Le rapport suivant doit constater l'intégration réelle, pas la présumer.
