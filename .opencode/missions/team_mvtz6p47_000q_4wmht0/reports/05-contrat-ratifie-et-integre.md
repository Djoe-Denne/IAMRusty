# Contrat Linux intégré — passage à l'implémentation k6

## Objectif et résultat

Le socle k6 peut maintenant être implémenté sur un contrat ratifié et intégré. Le lead a accepté la V1 proposée par l'architecte sous la délégation long run approuvée. ADR0504 et contrat sont Accepted / Unimplemented : l'architecture est décidée, aucun résultat fonctionnel n'est revendiqué.

## Récupération vérifiée

Le déplacement du worktree était empêché par un conflit de partage Windows confirmé (Win32 32), pas un défaut d'ACL. Aucun droit élargi, processus inconnu arrêté ou déplacement forcé. L'alternative de récupération manuelle proposée par l'outil natif a été exécutée par patch dans le checkout principal.

Avant ratification, les deux documents récupérés étaient strictement identiques octet pour octet aux originaux et à la sauvegarde, avec blobs Git identiques. Le lead a ensuite ajouté uniquement la ratification, les statuts cohérents et l'index ADR. Le commit ciblé a réussi avec le véritable hook cargo fmt intact et exécuté. Ce n'est pas une fusion native de commit auteur : les deux branches auteur n'avaient aucun commit nouveau ; leur proposition documentaire est récupérée et conservée explicitement dans le commit principal.

## Suite

Libérer l'auteur unique du socle/CLI/tests/workflow au HEAD intégré. La revue indépendante et la qualification restent différées jusqu'à intégration réelle du code. Cible Linux et bail contemporain restent à vérifier ; leur absence interdit HTTP PASS, mais pas les contrôles indépendants de l'outillage et de Chromium. Aucun nouveau GO de jalon demandé.

## Preuves et limites

- Commit intégré : c9b537c9c84f11dbdaf0178a1d19666de9261989 sur master.
- Trois fichiers conservés : docs/adr/0504-validation-fonctionnelle-linux-k6.md, docs/contracts/k6-functional-linux.md et docs/adr/README.md. Aucun WIP étranger staged.
- git diff --cached --check exit0 ; hook cargo fmt --all -- --check exit0 lors du commit réel. Pas de test k6/Python/Chromium exécuté.
- Blobs récupérés avant ratification : ADR083ccdcf92c700579ac375459e210f04324f325f ; contratea7e7bbba0345d205c2a52d7e003a9d6e8f59b50. SHA256 et index sauvegardés au ledger.
- Worktree source et index intacts, sauvegarde récupérable vérifiée ; aucune suppression effectuée. Ne pas l'annoncer nettoyé.
- SDK et Obsidian étrangers préservés ; un delta de mémoire Serena architecture/e2e-linux-adr-0504.md observé séparément n'est pas inclus dans ce commit.
- Aucun runtime créé/redémarré, aucun pull, aucun dispatch GitHub ni push à ce jalon.

## Préflight indépendant pendant l'implémentation

L'auteur outillage pool-4 a été admis et a reçu le HEAD intégré, le ledger et ses limites. Les tâches revue/qualification restent différées. Un membre architecture repris nativement pendant la récupération a été informé du contrat intégré ; son éventuel delta sera examiné avant disposition, pas jeté.

Le Docker Desktop local était déjà actif. Inventaire read-only depuis le contexte local desktop-linux : un seul conteneur préexistant running, du cluster aiforall-local ; aucun conteneur du laboratoire aiforall-local-full retenu au contrat n'est observé. Aucun runtime créé ou adopté. La cible HTTP autorisée n'est donc pas établie à ce stade ; les parties outillage et Chromium continuent indépendamment, sans lui substituer le cluster préexistant.
