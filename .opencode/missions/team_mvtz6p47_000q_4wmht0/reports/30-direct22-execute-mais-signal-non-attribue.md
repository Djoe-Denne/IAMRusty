# Direct22 exécuté : un signal reste à attribuer, pas une cause commune prouvée

## Résultat

Le garde privé a autorisé un seul diagnostic direct22, corrélé au refus k6 du même job. Chromium y termine par signal, mais sans la catégorie « No usable sandbox » observée sur24.04. Le signal n'est pas encore attribué : il peut provenir d'un arrêt natif ou d'une intervention du superviseur. Dire simplement que Chromium a crashé serait prématuré.

Les118tests Linux passent sans skip sur chaque plateforme. Les outils/versions/inspections réussissent. k6 termine avec la catégorie officielle SCRIPT_ABORTED sur les deux hôtes, avant le corps du scénario ; ce n'est pas un seuil échoué. Le contrôle des enfants rapporte cleanup PASS. HTTP/Chromium et cleanup parent restent INCONCLUSIVE ; aucun PASS global n'est acquis.

## Faits distincts

-24.04 : restriction AppArmor1 ; direct NO_USABLE_SANDBOX_REPORTED, signal et refus de confidentialité conservé.
-22.04 : restriction0 ; direct NATIVE_CAUSE_UNCLASSIFIED, signal, confidentialité PASS, aucun artefact natif publié.

Il serait incorrect d'assimiler ces deux issues à la même cause sans une observation supplémentaire. Les catégories actuelles ne distinguent ni le signal précis, ni un arrêt demandé par le superviseur, ni la famille du message natif non classé.

## Recherche après cet essai

Le guide Puppeteer décrit des échecs Crashpad liés aux répertoires de configuration/cache non inscriptibles. Notre environnement crée déjà HOME et XDG_CONFIG_HOME privés ; cette piste est connue, pas mesurée ici. Le ticket officiel runner-images14475 porte sur libXtst manquante avec Chrome151 et est annoncé corrigé : il ne prouve pas notre panne156, d'autant que les sondes de version de notre binaire passent. Aucun paquet ou option GPU/sandbox n'est changé sur la base de ces seules ressemblances.

Les recherches sur des crashes headless renvoient aussi des incidents anciens ou d'autres versions ; ils ne servent pas de preuve causale. Une analyse experte bornée doit maintenant proposer le plus petit diagnostic discriminant, en examinant aussi le chemin d'arrêt du superviseur. Recommandation : distinguer d'abord arrêt natif, intervention du parent et échec de témoin, plutôt que multiplier les changements d'environnement ou arguments à l'aveugle.

## Suite

Pas de nouveau lancement identique. Consulter un expert technique en lecture seule sur les sources et ces observations, sans modification de contrat/pin/scénario/garanties, pour choisir une prochaine observation minimale. Les décisions structurelles éventuelles restent à l'architecte avant code ; aucune exception AppArmor ou publication de flux bruts n'est autorisée par cette consultation.

## Annexe technique

- Publication vérifiée `ed87f65be6bb24380d5c6c84920ab1b6be35607d`, push exit0 et ls-remote identique. Source0601 inchangée après revue sécurité PASS.
- Run [40979730197](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40979730197), même HEAD/push ; jobs123816486171/123816486350 terminés failure. Tests24 :118/0.956s/OK ; tests22 :118/1.826s/OK, aucun skip.
-22 k6 : counters1/0/0/1/0, summaryVALID, nativeNONZERO/k6_exitSCRIPT_ABORTED, SANDBOX_UNPROVEN/exit2, privacy et enfants cleanup PASS. Son résultat privé courant a permis le direct ; parent_browser_run_id du direct correspond exactement au run_id k6.
-22 direct : nativeSIGNAL/terminationEXITED/NATIVE_CAUSE_UNCLASSIFIED, SANDBOX_UNPROVEN/exit2, privacy et enfants cleanup PASS. Le champ hérité comparison.browser_launch=NOT_STARTED ne décrit pas l'invocation directe ; direct_observation et la corrélation prouvent qu'elle a eu lieu. La ligne Direct22 NOT_STARTED visible dans le préambule shell est le code affiché, pas une branche exécutée.
-24 : SCRIPT_ABORTED avant le cas, directNoUsableSandbox/SIGNAL/PRIVACY_FAILED ; cleanup enfants PASS, upload skipped. Parent runtime INCONCLUSIVE partout.
- Reviewer15 arrêté normalement après persistance ; native merge sans branche. Aucun runtime local créé, ressource étrangère arrêtée ou arbre temporaire supprimé.

Sources consultées après le run :

- https://github.com/actions/runner-images/issues/14475 (libXtst/Chrome151, fermé ; commentaire renvoie à14481)
- https://raw.githubusercontent.com/puppeteer/puppeteer/main/docs/troubleshooting.md (section Running in read-only containers)
- https://github.com/chrome-php/chrome/issues/649 et https://github.com/puppeteer/puppeteer/issues/11023 : pistes Crashpad issues des recherches, pas causes établies ni remèdes adoptés.
