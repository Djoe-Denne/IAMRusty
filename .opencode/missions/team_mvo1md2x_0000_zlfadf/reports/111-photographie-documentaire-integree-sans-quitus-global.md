# Documentation actualisée pendant l'enquête de reprise

Les deux documents existants de plateforme et de contrat disposent maintenant d'une photographie factuelle au27octobre. Le lead a intégré les trois propositions successives de la même auteure en un état consolidé, sans importer son ancien staging ni modifier les critères d'acceptation.

Les documents distinguent les preuves historiques vertes, le NoOp validé, les échecs97/105, la correction temporelle intégrée et revue, puis le nouvel échec109 après reconstruction. Ils précisent que le verrou temporaire n'est qu'une hypothèse, que le diagnostic110 est autorisé mais sans résultat, et que publication/pin/acceptation globale restent bloqués. Les bilans de cleanup restent séparés par campagne.

Vérification : `git diff --check`0. Le fichier de test conserve exactement son empreinte revue6ac7be0e ; aucun changement de production, SDK ou ADR. Seuls les deux documents, le test corrigé et la différence de checkoutSDK préexistante figurent dans le diff intégré. Aucun commit ou publication effectué à cette étape. Les originaux/staged de l'auteure restent préservés.

## Chemins et limites

- `docs/platform/events-outbox.md` : opt-in réel des configurations et photographie Telegraph.
- `docs/contracts/telegraph-notification-outbox.md` : section10 factuelle ; AC1–AC7 et décisions structurelles inchangés, aucun statut ADR promu.
- Props archivées chez l'auteure : proposed-docs.patch puis105-106 puis109-110 ; ce dernier SHA256 `b13abaf95e281001c1c48d950ab5312baf45233636051a461b39d5a0085fbc8e`. Lead consolide et met le titre/date/lien d'ancre au27octobre ; il formule la reconstruction comme une gate encore rouge plutôt qu'une garantie globalement livrée.
- Cette intégration documentaire n'est ni un test ni une permission de nouvelle exécution. Enquête hard-implementerfresh en cours, effort élevé demandé et confirmation native toujours attendue ; run110 non consommé.
