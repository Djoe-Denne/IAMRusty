# Contrat d'identité intégré et ratifié — correction libérée

## Résultat utile

Le contrat qui distingue les autorités jusqu'aux permissions est intégré et ratifié sous la délégation approuvée. Il conserve les parcours organisation légitimes au lieu de les interdire globalement. L'ADR reste **Unimplemented** : accepter la cible n'est pas annoncer une correction ou une campagne réussie.

Le consommateur de ce contrat a été publié après intégration réelle. Le mécanisme natif l'a affecté à un hard-implementer existant/nouveau pool ; le spawn explicite concurrent a été refusé sans doublon. Son plan a été récupéré et examiné ; les attributions ciblées supplémentaires sont consignées au contrat, sans refonte de périmètre.

Actualisation de reprise : le plan a été explicitement approuvé et la production SDK est engagée. L'annexe d'attribution est intégrée par le commit `1ad1e9f`. Le premier créneau exclusif Cargo est réservé à l'auteur identité pour fmt/check SDK natifs sans runtime ; aucun résultat n'est encore acquis. Le contrat distant a aussi été préparé par un pool auto-affecté : les deux propositions sont comparées avant une ratification unique, sans publier deux consommateurs.

La [mise à jour utilisateur de délégation](../long-run-delegation.md) est conservée dans les notes et a été diffusée aux membres actifs, puis au consommateur nouvellement admis. Elle n'a entraîné aucun changement de la politique approuvée, des modèles, de la QA ou des permissions, ni config/start supplémentaire.

## Décision d'architecture et vérifications lead

L'issuer exact est conservé dans Subject, caches, projections et suppressions. Le fil UUID historique est réservé à une plateforme attestée/configurée ; les autres autorités ont un sujet injectif qualifié. Pas de fallback, alias par suffixe, double grant permissif ou conversion d'un refus authentifié en anonyme. Les interfaces UUID-only refusent localement les identités incompatibles ; le public réel et les grants propres aux organisations restent positifs.

Le lead a recoupé l'alias Hive par suffixe `/iam` dans les corps Serena. La fixture OpenFGA épingle v1.5.0 ; son module API résolu autorise TupleKey.user jusqu'à512 octets, contre387 au maximum pour le sujet proposé. Ce contrôle de sources n'est pas une preuve de serveur exécuté. Les tests de collision exigent plusieurs autorités reconnues et leurs propres grants/révocations, pas une simple401 qui masquerait le défaut.

La ratification a été matérialisée dans ADR0314, contrat, index, pointeur wiki et digest Serena au même tour, sans modifier les Accepted antérieures. Un commit lead limité à ces cinq documents rend le contrat disponible dans les futurs worktrees ; les modifications étrangères Obsidian/SDK sont restées hors staging/commit.

## Livraison documentaire : échecs fmt correctement bornés

Le hook `.githooks/pre-commit` inspecté lance uniquement `cargo fmt --all -- --check`, inconditionnellement, même pour les docs. Les auteurs identité et remote ont chacun rapporté une tentative involontaire échouée Windows OS206. Ce sont des échecs d'environnement, pas des ROUGES fonctionnels ni des fmt PASS.

Après contrôle des seuls fichiers documentaires et whitespace, une exception éphémère pour l'invocation Git de livraison a été autorisée, sans modifier hook/config/permission persistante. Le gate Rust intégré n'est pas dispensé. Les auteurs ont reçu l'embargo Cargo y compris par hook sans créneau lead.

Identité : livraison auteur vérifiée, shutdown demandé puis team_merge natif réussi. Le nettoyage du worktree est resté pending et sa branche est conservée : pas de suppression forcée ou de merge rejoué aveuglément. Remote : contrat lu, seul fichier staged vérifié, correction de la mention Cargo demandée, commit auteur maintenant présent et arbre propre ; son intégration est la prochaine opération.

## Plans, disponibilité et ressources

Envoy : l'auteur avait prouvé un refus external_directory et n'avait pas écrit dans le canon. Le lead a utilisé session_move officiel sur la même session, sans changer les permissions, puis approuvé son plan. Un résultat natif ultérieur indique les deux configurations et quatre tests statiques ROUGE/VERT livrés ; commit différé pour respecter l'embargo Cargo. La revue du diff et de ces preuves précède sa livraison/intégration ; aucune preuve réseau Envoy revendiquée.

Identité : worktree application propre à l'entrée documentaire intégrée, puis vraie worktree SDK isolée sous son gitlink, branche de mission au pin initial. Le checkout SDK canonique sale n'a pas été édité, copié ou repointé. Le consommateur a confirmé l'accès et la propreté des deux arbres, puis attendu l'examen du plan avant production. Les targets SDK ont leurs chemins réels attribués ; policy sentinel et plumbing de trois consommateurs supplémentaires sont bornés. IAM reste un broker d'intégration après remote/SDK, pas un fichier à deux propriétaires concurrents.

Navigateur : deux contrats proposés ont été récupérés dans la session native, non committés. Ils conservent l'entrée HTTP loopback et le vrai navigateur, avec un vrai connecteur GitLab et un fournisseur synthétique ; aucune nouvelle ADR requise. Leur examen continue, sans demander un accord utilisateur de catégorie ni qualifier par avance le futur runtime.

Aucun Docker/Kind/WSL de mission créé/redémarré, aucune IT ou campagne comportementale exécutée à ce jalon. Aucun créneau Cargo n'est attribué à un auteur ; aucun clear_runtime8B. La suite reste autonome, sous barrières d'intégration et revue indépendante finale.

## Annexe de preuve

- Ratification intégrée : commit lead `6a3268b07081e3a62a94567f056c86ebc086f0f4`, cinq fichiers documentaires seulement ; hook persistant `.githooks` conservé. Aucun push du dépôt applicatif.
- Livraison auteur identité : `d159c7bd6f07e2200bf3f0cc250f451d4175ab5b` ; merge natif effectué, cleanup worktree pending.
- Producteur accepté : `task_mvsf2g5q_0003_qqrmnr` ; consommateur publié `task_mvsgu86t_000u_a68cwv`, propriétaire pool-3, session `ses_e7c29fc1bffeD3UMz0JD5Lkkpi` ; refus de spawn parce que déjà affecté, pas de second auteur.
- SDK secondaire : `.../ensemble-04d9a2f8-d9lbvu-pool-3/rustycog`, branche `mission/authz-principal-sdk-pool3-20261028`, base d60a88b. Gitlink application encore d60a88b ; publication et sélection future au lead.
- Remote : commit auteur `794f0221c079933bbc56141278769780c92cf680`, arbre propre vérifié ; shutdown demandé.
- Sources officielles : https://raw.githubusercontent.com/openfga/openfga/v1.5.0/go.mod → api/proto `v0.0.0-20240201160513-05de9d8be3ee` ; https://raw.githubusercontent.com/openfga/api/05de9d8be3ee/openfga/v1/openfga.proto (TupleKey.user max_bytes512).
- Recherche initiale sans résultat corrigée par accès aux sources officielles épinglées. Un patch documentaire n'a pas été appliqué à cause d'un contexte de ligne inexact ; absence de modification partielle vérifiée, puis patch ciblé corrigé. Pas de panne d'outil ou de preuve fonctionnelle déduite de ces incidents.
