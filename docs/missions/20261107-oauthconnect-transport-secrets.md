# OAuthConnect — transport OAuth et secrets de démarrage

## Mandat et base

GO utilisateur du 2026-11-07, suite du lot configuration/projection publié dans `3ba3e3ca21210ad5ae6d4bd69ecd8e117298c077`. Sous-agents directs, sans mission Ensemble ni PM. Vérification proportionnée dans la continuité du plan QA T3/G-3 et T6/G-6 ; RED comportemental avant GREEN.

Canon : ADR0413 Accepted/Partial, `docs/contracts/oauthconnect-multi-instances.md` §§2.1–2.3,3–4,6–8 et annexe JSONPath. Réutiliser le loader, les types et les114tests précédents sans changer les règles acceptées ni rouvrir S-1. SDK/pin inchangés.

## Résultat borné

- Transport OAuth configurable : authorize v2, token POST form et APIs GET JSON fixes ; branchement à la projection pure et à son budget. Allowlist redirect exacte avant I/O, S256 selon capacité et contexte nonce conservé dans la frontière S2S sans paramètre OIDC ajouté au vendor OAuth.
- Clients HTTP bornés/injectés : aucune redirection,10s total/3s connexion, corps décompressé lu au plus1MiB avant parsing, aucune relance du code, méthodes d'auth explicites. Même définition API appelée une fois par complétion et réponses jamais cachées entre opérations.
- Résolution des secrets direct/OpenBao via mécanismes SDK existants au démarrage, par instance, erreurs génériques/redaction, aucune clé de secours ou catalogue amputé. Réemployer le comportement GitHub/SDK acquis et ses fixtures HTTP sortantes.
- Raccord minimal `setup` permettant de construire les clients OAuth et de fournir leurs clés résolues à un consommateur de composition ultérieur. Pas de listener, binaire, route publique ou registry opérationnel annoncé avant support de tous les profils configurés.

Hors lot : OIDC Discovery/JWKS/ID token/UserInfo, nonce transactionnel/raccord IAM, deployment/Compose/TLS mesh/Kind, retrait legacy, claims riches/actions et E2E externes. La résolution des secrets peut traiter les déclarations des deux profils, mais une construction demandant un client OIDC non implémenté doit échouer explicitement, **jamais ignorer une instance ni publier un catalogue partiel**. Si cette borne requiert de changer la cible ou un port structurel, consulter architecte avant code concerné.

## Ordre et parallélisme

1. Socle minimal d'API compilables et refusantes : client OAuth, transport injecté, types de secrets résolus redacted et petites fonctions de composition. Pas de comportement fonctionnel anticipé avant les tests.
2. Intégration/versionnage des APIs et compilation au canon avant auteurs consommateurs isolés.
3. Deux branches disjointes après cette barrière : tests/RED/production du transport OAuth ; tests/RED/production de la résolution des secrets. Interface et propriété des fichiers explicites, parent seul modifie workspace/lock. Aucune compilation concurrente.
4. Intégration des deux producteurs puis tests du raccord réel (client, résolution, réponses et projection). Consommateurs ne travaillent pas sur des branches non intégrées. Reviews indépendantes adaptées à authn/secrets/HTTP, briefing persisté avant correction.

## Preuves attendues

- Vrai client/setup utilisé, WireMock uniquement pour collaborateurs HTTP sortants (vendor/coffre). Pas de mock de usecase/service ; réemploi des fixtures existantes, pas de nouveau harness global. Tests natifs Windows, un Cargo, jobs24/targetE:, aucun Docker/Kind nécessaire a priori.
- Positifs authorize exact et complete tokens+profil ; code/verifier seulement au bon moment, pas d'appel v1, erreurs normales sans réponse partielle. Secrets distincts par instance et mapping sur hôtes arbitraires.
- Négatifs : redirects proches mais non exacts avant I/O, HTTP malformé/401/403/429/5xx/HTML/JSON invalide/redirect, timeout, corps trop grand y compris décompressé, type token invalide/vide, erreurs email non avalées, isolation opération/instance et zéro retry du code.
- Secrets : mode exclusif, références/token-file SDK et bornes conservés, coffre indisponible/clé manquante/valeur vide/HMAC court provoquent refus global ; zéro fallback direct/env. Les fixtures OpenBao WireMock ne seront pas annoncées comme preuve d'un vrai coffre.
- Conserver RED exécuté puis mêmes oracles GREEN. Compilation/fixture indisponible n'est pas RED, suite filtrée ou non exécutée n'est pas PASS. Après intégration : tests ciblés, check/fmt/Clippy strict selon targets réels et régressions affectées. Plan QA de détail réconcilié aux API avant tests.
- Ressources runtime : aucune création prévue. Avant run réel, charger lifecycle/test-evidence et vérifier les effets des fixtures. Toute nécessité de Docker/Kind ou d'un nouveau harness est un changement à arbitrer, pas une activation implicite.

## Clôture

Photographie ADR/contracts/QA mise à jour seulement après preuves, tous livrables durables intégrés/committés/poussés surmaster et référence distante vérifiée ; worktrees temporaires sauvegardés et retirés propres par mécanismes natifs. ADR0413 restePartial. La fin de ce lot ne qualifie ni OIDC/IAM, ni le boot complet avec Discovery, ni les E2E/cutover.
