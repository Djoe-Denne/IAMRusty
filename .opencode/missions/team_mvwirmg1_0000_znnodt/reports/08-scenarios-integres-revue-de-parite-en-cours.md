# 8. Scénarios intégrés, revue de parité

## Objectif et état

Les scénarios d'authentification et les témoins HTTP du certificat sont intégrés dans master. Une revue indépendante bornée va examiner leurs garanties pendant que l'auteur du parent termine le raccord Windows. Aucun test de robustesse du framework ajouté.

## Vérifié

Deux fichiers seulement,267lignes, identiques byte-à-byte aux fichiers livrés et relus. L'auteur a vérifié leurs APIs contre le tag officiel k6v2.3.0 et rapporte deux contrôles de syntaxe JS réussis sans exécution/import. Le lead a vérifié l'identité des fichiers ; diff--check et hook natif fmt du commit passent. Les preuves statiques auteur restent attribuées à leur portée et ne sont pas rejouées par principe.

Le navigateur exige26checks fermés et les dix requêtes/réponses IAM correspondant à sept appels positifs et trois refus. Chaque témoin certificat fait un seul GET sans suivre la redirection, avec deux checks et statut303/503/303 selon la phase. Ces décomptes doivent encore être confrontés aux routes/configurations chargées et au vrai run ; ils ne prouvent pas à eux seuls PKCE, les compteurs Envoy ou la restauration parent.

## Suite et limites

Revue indépendante des oracles intégrés, puis corrections bloquantes si nécessaire. Le parent reçoit les interfaces exactes et continue sa tâche non encore acceptée. Sa disponibilité ne doit pas être remplacée par un rapport de helpers statiques. Aucune création du laboratoire avant intégration finale et gates applicables/réutilisés.

Pas de k6/Chrome, HTTP, nouveau build ou ressource Docker dans cette étape. Le commit est local, publication distante non encore effectuée ; la mission n'est pas clôturée.

## Annexe

Commit0c1e21d ; candidat consolidé result_mvwk9hzy_000p_uxpjfs, session ses_e6d774369ffe6U5Hiz4iaD7QGc. SHA256oauth775628c968486ad69c8412722899aeaa65651e9ca46c835d11894f16c15b2992 ; SAN37f1f8f69cbca45a1dfc21bcdb853e42df69f6352e060373f2289a8c1b2e5368. Briefing de revue .cursor/review-briefings/20261031-k6-auth-scenarios-parity.md.

Auteur code sans commit propre : récupération par patch avec identité vérifiée, commit canonique puis tâche auteur complétée et arrêtée. team_merge confirme aucun commit de branche à fusionner ; son arbre non committé est conservé jusqu'à disposition finale, aucun travail durable perdu.
