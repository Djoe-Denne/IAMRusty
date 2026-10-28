# Rapport 80 — reprise et régressions de compatibilité à corriger

## État vérifié

AI HEAD36357f633e40c825830c124596a8a6dd674de655 ; SDK local9e3714948e7b815ee24e507d25a9c6d1e3dc911d propre ; seul `M rustycog` dans le superprojet. Aucun pin final ou push effectué. Les trois tests callers verts du rapport79 restent des preuves ciblées valides, pas une acceptation globale.

La relecture Rust annonce RP-1/RP-2 résolus, GO_WITH_COMMENTS ; retransmission du paquet complet2002Z demandée pour persistance sans nouvelle analyse. T4 termine son paquet indépendant de compatibilité. Treize preuves brutes owner10 et trois callers copiées et hash-vérifiées sous `.tmp/telegraph-review-inputs/final-http-proof/` pour sa lecture locale.

## Findings T4 préliminaires, non encore corrigés

- Port0 : `TelegraphApp::run` transmet config brute à l'owner (app.rs165–172), qui lie config.port (owned_server.rs425–430). Le legacy résout actual_port ; le chemin public de test readiness/baseURL doit rester compatible. Le port0 direct de l'owner et local_addrs restent intentionnellement éphémères : ne pas casser cette API pour réparer le caller.
- HTTP/2 : nouveau builder owned_server.rs552–555 ne configure pas Extended CONNECT. T4 le compare à Axum0.8.9 qui l'active ; absence de préservation du protocole à traiter sans backend/upgrade neuf.
- Fixtures : RunningApp conserve désormais l'owner HTTP hors du vecteur tasks (38–61). Les trois assertions historiques de parents1/3 deviennent incompatibles avec0/2. Conserver une assertion explicite de propriété HTTP et des tâches métier, pas seulement supprimer les compteurs pour obtenir vert.

Lectures ciblées exactes lead corroborent les trois chaînes ci-dessus ; pas de nouveaux tests ou rouges exécutés à ce stade. Paquets finaux à persister avant correctifs. Aucun défaut de deadline déjà résolu ne doit être rouvert par simple absence de couverture ; aucun de ces nouveaux défauts n'est effacé par les tests précédents.

## Suite bornée

Contrat HTTP impose déjà compatibilité callers/protocoles ; corrections et témoins ciblés restent dans cette surface autorisée. Tests séparés de production, pas nouveau harness, protocole désactivé, oracle affaibli ou recampagne globale. Les scénarios IT existants empêchés par les anciennes assertions nécessiteront une vérification honnête avec fixtures existantes, après préflight/runtime autorisé. Aucun runtime démarré pour cette reprise statique. C-1, publication et pin final restent ouverts.
