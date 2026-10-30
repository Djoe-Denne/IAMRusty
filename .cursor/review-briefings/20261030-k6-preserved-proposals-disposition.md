# Disposition des cinq documents historiques non identiques à l'historique master

## But et périmètre

Travail documentaire READONLY indépendant pendant l'auteur du conducteur.
Préparer la clôture exigée par AGENTS sans jeter une précision utile ni réintégrer
une ancienne proposition à la place d'une décision Accepted. Aucun retrait ou
changement du contrat du conducteur, source, index, Git refs ou runtime.

Six arbres inactifs sont sauvegardés, fichiers/index/patches/HEAD/status avec
SHA256 et comparaison des originaux avant/après :
`C:/Users/djden/source/repos/AIForAll/.tmp/k6-inactive-worktrees-20261030-c268b9bdec094ccbb00b3e0d01e87474/`.
Arbre13 actif EXCLU. Chaque arbre inactif et six références natives préservées
ont zéro commit unique contre master. Le backup local ne remplace pas le push
des livrables durables et n'autorise aucune suppression.

## Documents à examiner

Dans les sous-dossiers `files/` des sauvegardes ci-dessus :
- pool-3 : docs/adr/0504-validation-fonctionnelle-linux-k6.md et
  docs/contracts/k6-functional-linux.md ; alternative architecturale antérieure.
- pool-9 : docs/contracts/k6-functional-linux.md ; proposition du diagnostic parent.
- pool-12 : docs/contracts/k6-functional-linux.md et
  docs/contracts/k6-chromium-causal-observation.md ; propositions/photographies
  précédant les ratifications et actualisations du lead.

Les 25 autres fichiers sauvegardés (arbres10/4/7) sont retrouvés intégralement
dans l'historique atteignable de master, après seule normalisation CRLF→LF pour
comparer les blobs Git. Ce n'est pas une preuve que les cinq docs soient absents
ou inutiles : leurs contenus ont été récupérés partiellement ou ratifiés avec
statuts différents, documentés dans le ledger et les rapports.

Canon à comparer : fichiers ACTUELS du principal, HEAD22f7a2e ou successeur
documentaire compatible ; `docs/adr/` et `docs/contracts/` priment sur les candidats.
Réutiliser les faits de récupération connus : diagnostic parent ratifié,
comparaison22 seule ratifiée sans exception AppArmor, canal privé conditionnel,
opération réelle et amendement pipes récupérés à l'identique puis ratifiés.

## Livrable attendu

Pour chaque document : disposition explicite avec sections/paragraphes récupérés
dans le canon, proposition remplacée ou non ratifiée à conserver comme telle,
et **seulement** une éventuelle précision utile réellement absente à proposer au
lead avec son statut. Ne pas reconstruire tout l'historique ni faire un audit
global. Diffs ciblés et sources du ledger suffisent ; pas besoin d'ouvrir les
25 fichiers source déjà retrouvés dans master.

Rendre rapport final autonome avec correspondances vérifiées, limites et texte
de précision manquante si nécessaire. Pas d'écriture dans les docs/ledger partagés,
nouvelle décision Accepted, code, worker, WSL, Cargo, CI, installation ou suppression.
Le lead persistera la disposition ; un document remplacé ne sera pas réappliqué
aveuglément et aucune ancre publique ou exception AppArmor ne sera promue.
