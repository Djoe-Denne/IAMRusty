# Rapport 68 — déblocage minimal d'un test SDK préexistant

## État et preuves

E2 écrit six témoins owner SDK, mais ils ne se sont pas exécutés : compilation `--lib` échouée E0599 dans `rustycog-http/src/jwks.rs:693`. Lead recoupe sdk-owned-1.log (erreur64–71) et les sources Git : la même ligne `expiry.saturating_sub(Duration::from_nanos(1))` existe dans129dcc1 eted1809d. Le code fautif est dans le test `negative_cache_is_bounded_and_expires_at_boundary`, pas dans la production. `std::time::Instant` n'a pas cette méthode.

Ce n'est ni un rouge fonctionnel C-1, ni une régression owner démontrée. Aucune production corrigée par l'auteur tests. E1/mTLS sont des cibles indépendantes autorisées et peuvent continuer ; aucun résultat vert annoncé ici. Revue concurrence indépendante non concernée par cette erreur de compilation.

## Arbitrage demandé

Le fichier jwks.rs n'appartient pas à l'ownership des nouveaux tests owner explicitement bornés. Proposer une extension mécanique **au seul test préexistant**, pour permettre la compilation du même target SDKlib déjà autorisé : remplacer l'appel invalide par `expiry.checked_sub(Duration::from_nanos(1)).expect(...)`, conserver la vérification à une nanoseconde avant expiration et toutes les assertions à l'échéance. Aucun changement de cache/JWT/authentification en production, de target/manifeste/dependance ou d'oracle.

Recommandation : autoriser ce correctif test-only minimal par le même test-engineer, puis exécuter ce test JWKS ciblé ainsi que les six tests owner déjà prévus, sous l'unique slotCargo Windows24. Conserver la trace E0599 comme précondition cassée, pas rouge métier ; SHA source+tests/fix à tracer avant publication.

Alternative : ne pas toucher au test hors ownership, maintenir SDKlib bloqué et rendre cette couverture non exécutée. Les autres cibles peuvent produire des preuves partielles, jamais PASSglobal. Aucun nouveau harness ou assouplissement d'AC7 proposé.

## Réponse utilisateur

L'utilisateur choisit **Autoriser le correctif test**. Autorisation limitée au test JWKS décrit, remplacement checked_sub+expect et vérifications ciblées de ce test puis owner déjà prévus ; aucune modification de production ou d'oracle. Même auteur E2, sans nouvelle chaîne QA ni nouveau target. Le résultat réel reste à exécuter et à recouper.
