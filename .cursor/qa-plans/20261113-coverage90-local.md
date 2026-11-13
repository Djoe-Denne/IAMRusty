# Coverage 90 local — contrat QA proportionné

## Mandat explicite utilisateur
- Après résultats/nettoyagevalidés,useraautorisécommit/pushsurbrancheprincipaleeffectiveMASTERetnotificationdelasessionCI. Limitesinchangées :passeulementd'autreslivrablesétrangers/pasworkflowCIéditéicipoursimplementpublier ;interdictioninitialecommit/pushhistoriquelevéeparceGOdistinct.
- Extension approuvée après preuves ROUGES : corriger LOCAL les4défautsproduction(IDORcancelHive,paginationHive,replacegrantsHive,mapperTelegraph),toujourssanscommit/push/CI. Lecontrattests-onlydespremiersauteursrestaitvalideàl'époque ; cetaccordn'autoriseaucunautrefixprod.
- Travailler LOCAL uniquement : aucun commit, push, appel de déclenchement CI ni modification des workflows. Un autre agent possède la CI/E2E.
- Exclure uniquement le harness explicitement TEST-ONLY (`crates/apparatus-contracts/src/harness.rs`). Pas d'exclusion de DTO, entités, commandes ou logique de sécurité.
- Implémenter les tests faciles/utiles et les IT à forte valeur du triage. Différer maintenance runtime signée, origine trusted/mTLS positive et les autres setups réellement complexes.

## Invariants et frontières
- Code production inchangé par les auteurs de tests. Tout bug découvert est rendu ROUGE au lead, sans affaiblissement d'assertion ni correction production silencieuse.
- ADR0200/0201/0202 Accepted : harness HTTP existant, migrations/Postgres réel, OpenFGA réel, WireMock seulement HTTP sortant, queues opt-in, #[serial] si fixtures globales. Unitaires purs aux règles/validations/mappings ; ne pas les étiqueter IT.
- Privilégier IT de parcours quand raisonnable ; unités ciblées pour tables de validation/mapping/transitions sans infrastructure.
- Pas de tests getters/copies de champs artificiels. Assert comportement, code fermé, état persistant, absence d'effets indus et refus utiles.
- Pas de nouvelles fixtures infra ni harness concurrent. Réutiliser fixtures et Common de chaque service. Auteurs : nouveaux fichiers de tests seulement, pas de Cargo.toml/common partagé sans arbitrage.

## Découpage indépendant et ownership
- A IAM unités : nouveaux fichiers sous IAMRusty application/domain/http tests ; validations commandes, erreurs HTTP, auth simple/DTO/conversions. Pas de crypto/provider live compliqués.
- B Hive+Manifesto unités : nouveaux fichiers sous application/domain tests ; transitions entités, règles pures, validations/mappings commandes et DTO.
- C Telegraph+contrats unités : nouveaux fichiers sous Telegraph domain/application/infra tests et crates/apparatus-contracts/tests ; erreurs, règles de communication, validation/parsing de contrat. Autres crates pures si directement utiles sans infra.
- D IT de services : nouveaux fichiers sous services/{Hive,Manifesto,IAMRusty}/tests ; extensions de scénarios HTTP+DB existants, permissions/refus/pagination/rollback. Pas des répertoires tests des sous-crates possédés par A/B/C.
- Lead : exclusions Sonar, intégration, lanceur IT (sélections exactes si nécessaire), instrumentation/mesure, QA/reviews, ressources. Aucun auteur ne compile : slot Cargo réservé au lead après intégration.

## Validation et preuve
- Correctifsprod : conserverles4oraclesrougesexistants,sansaffaiblissement. Architecturedu replace/UoWàconfirmeravantimplémentation ; éviterdelete+insertnonatomic etnepaschangermodèleévénement/FGAimplicitement. Re-testidempotence/réduction/rollbacksiport/UoWchange ; revueindépendantecorrectnessetsecurityscopeHiveauthz. Réutiliserpreuvesunitésnonaffectées ; exécuterITfinalesaprèsintégrationtouscorrectifs.
- Windows natif, 24 jobs du dépôt, target E:, un seul Cargo et aucun pendant les éditions des autres lots.
- Après auteurs : vérifier fichiers, intégrer avant consommateurs/QA ; fmt/check/clippy ciblés puis unités puis IT finales via it-native, cleanup tracé par IDs.
- Mesurer avec instrumentation LLVM locale. Distinguer gain de tests, effet de l'exclusion, projection fusionnée baseline et couverture totale réellement rejouée. Aucun nouveau pourcentage Sonar distant annoncé sans analyse réelle (pas de scan CI dans ce lot).
- Reviews indépendantes bornées aux nouveaux tests et exclusions ; briefings persistés. Aucun pipeline automatique imposé.
- Livrable local non committé, plus handoff non secret et inventaire exact des commandes/résultats/gaps. L'absence d'IT ou un blocage d'environnement ne devient pas PASS.
