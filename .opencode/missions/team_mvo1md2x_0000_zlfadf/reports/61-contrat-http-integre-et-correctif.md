# Rapport 61 — contrat intégré, entrée du correctif HTTP

## Artefacts intégrés

L'architecte ne disposait pas d'outil patch ; il a remis un patch documentaire complet sans écriture shell de repli. Lead l'applique via outil dédié sur **quatre documents seulement**, vérifie les ancres et le diff, puis commit normal **64e91dde232603a8d2525439b6edd69e7cf9d75f** : ADR0503 Proposed/Unimplemented, contrat HTTP, index ADR, référence §5.3 du contrat Telegraph. +218/−2lignes, diffcheck et hook normal réussis, status propre. Aucun wiki ou digest MCP modifié ; ce dernier n'est pas une gate d'implémentation et reste à synchroniser si utile.

E1 intégré b291eee et rouge réel déjà recoupé ; contrat/API intégré64e91dd. Les deux préconditions sont donc satisfaites, sans Accept ADR ni clôture C-1.

## Lot prêt : production SDK et adoption

Hard-implementer fresh pour ce nouveau sous-lot SDK HTTP, distinct de l'auteur historique T3 : base AI64e91dd, SDK129dcc1 ; sources SDK isolées et identifiées avant mutation, jamais le checkout utilisateur frère sale. Préserver la pile et les protocoles, API additive du contrat, budget commun5+1s et annulation/jointure exhaustive. Surface fichiers du contrat§2, pas de refonte/harness/backend nouveau. Tous écarts structurels reviennent à architecte/lead avant modification.

Le worker doit d'abord proposer son emplacement source SDK et son raccordement de développement, avec propriété/HEAD/remote, sans transformer un commit local non publié en pin livrable. Gitlink final uniquement après tests, revue et publication du SHA exact. Pas de push implicite d'autres branches ou changements utilisateur.

Un seul propriétaire production pour SDK+adoption, car traits async/exécuteur/callers sont étroitement liés. Après livraison intégrée exploitable, test-engineer écrit les compléments QA du contrat puis revue indépendante bornée : ces consommateurs restent non publiés jusqu'à disponibilité réelle du code/API. E1 rouge historique immuable ; son raccordement vert sera adapté par l'auteur tests aux chemins réellement adoptés, sans baisser les oracles.

## Vérification et ressources

Le worker production peut compiler/formater les packages touchés sous slotCargo Windows24jobs exclusif, après préflight ; ne pas lancer de campagne Docker/IT/SQS/FGA ni écrire les tests manquants à la place du test-engineer. Livrer inventaire des protocoles/spawns, hypothèses/limites, commandes/résultats et sources SDK exactes. La validation HTTP ciblée reste proportionnée selon contrat§6 ; aucun pipeline global supplémentaire. T5 staged et findings T4 préservés.

Statut actuel : aucun correctif production HTTP encore écrit ; C-1 reste ouvert. Source SDK et mode de développement à vérifier dans le plan du worker avant mutation ; toute indisponibilité de tests/publication se rapporte honnêtement.
