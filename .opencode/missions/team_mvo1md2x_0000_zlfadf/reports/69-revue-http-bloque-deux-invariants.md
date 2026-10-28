# Rapport 69 — deux findings HTTP bloquants, témoins à exécuter

Revue fresh rust-perf livrée BLOCK sur AI65d2219/SDKed1809d local. Brief final persistant/indexé **avant correction** : `.cursor/review-briefings/20261026T1901Z-rust-perf-http-owned-concurrency-65d2219.md`. Lead recoupe owned_server.rs124–233 ; pas de test de ces findings exécuté par reviewer/lead.

- **HTTP RP-1** : premier join peut rendre erreur de deadline/non-observation puis un join ultérieur rendre et mémoriser Ok après récolte des cancellations. Il faut conserver le diagnostic historique sans abandonner la reprise de récolte.
- **HTTP RP-2** : timer de force uniquement dans le futur join emprunté ; annuler ce futur tout en gardant owner vivant suspend la force aux échéances déjà demandées. Le contrat exige sa progression indépendante du join et du coordinateur.

Les sévérités P0 du reviewer sont des blocages de correctness shutdown, pas des constats de soundness ou exploitabilité. Aucun finding ne concerne le RP-1 SQS historique clos129dcc1. Les autres contrôles de propriété n'effacent pas ces défauts ; aucun protocole/vert global validé par analyse statique.

## Suite dans le périmètre autorisé

Ne pas limiter le contrat au join continuellement pollé. Conserver API/exclusions/protocoles, reproduire les deux témoins dans le module SDK existant puis correction production bornée : diagnostic sticky séparé de la complétude ; mécanisme privé possédé de force aux échéances monotones, pas watchdog détaché. Détails locaux dans les invariants architecte, pas nouveau backend/harness ou autorisation silencieuse plus large.

E2 reste auteur tests indépendant de cette revue jusqu'à sa première remise. Le lead ne transmet pas ces conclusions à son contexte aveugle en cours ; les témoins additionnels après remise pourront revenir à la même session sans répéter toute la campagne. Aucun correctif produit/pin/push libéré à ce checkpoint. Compilation SDKlib bloquée JWKS préexistant est un sujet distinct, correctif test-only déjà autorisé humainement au rapport68.
