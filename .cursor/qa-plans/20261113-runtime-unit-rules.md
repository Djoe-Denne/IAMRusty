# Runtime — unités réelles sans Envoy

## Mandat
- Implémenter les règles unitaires maintenance/preuves après discussion utilisateur ; Envoy/mTLS reste une preuve E2E. Migrationsdéjàexcluespar**/migration/src/** : aucuneexclusionnouvelle,niproductionmétierretirée.
- Ne pas injecter un channel/authenticator/verifier retournant true/Ok pour bénir les données du test. Vraie implémentation cryptographique, vecteurs indépendants déjà présents ; règles pures testées directement, sans prétendre authentifier le transport.
- Codeproductioninchangéhorsdéclarationscfg(test)nécessaires ; pasrefactorpublic/port/dep/harnessnouveau. ToutbugréelresteROUGEàremonteravantcorrectifautorisé.

## Deux lots indépendants
- A : fonctions pures privées de maintenance.rs (command/ack/plan/action) et gardesfailclosedconfiguration (sansSQL ni fauxchannel), nouveau module tests adjacent et branchecfg(test). Les RuntimeMaintenanceFacts construites restent données, paspreuvepositivecanal.
- B : vrai PinnedRuntimeAdmissionProofVerifier, tests complémentaires de configuration/pinning, révocation de clé et bornes temporelles/canonicité en réutilisant les vecteurs RSA/JCS indépendants existants. Pasdecléprivéeconstante,nifauxvérificateur,nimTLSsimulé.
- Les tests existants apparatus_runtime_admission.rs doivent être inspectés pour éviter duplication. AucuneITSQLsansDBréelle ;helpersQueryResultSQLnonciblésparfauxDB.

## QA proportionnée
- Auteursisolés, aucunCargo/runtime/commit/CI ; leadintègrepuis1CargoWindows24jobs,targetE. PasDocker/Kind/installationCAprofilutilisateurpourcesunités.
- fmt/Clippy ciblés puiscargo test -p manifesto-infra --lib (ou ciblesexactespourtestsB selonplacement),régressionexistanteapparatus_runtime_admission sansruntime àdéduiredel'inspection.
- RéellepreuvepositiveRSApuisaltération/refus,révocationeffective,jamaisboolmock ; tempsfourniauverifiertestable sanssleep. Reviewerindépendanttestssuroracles/mutations,preuveencorevalidepasreruncérémoniel.
- Profils/logsE:/cargo-target/AIForAll/runtime-unit-rules-20261113/,LLVM_PROFILE_FILEexplicite siinstrumenté. Pasprojectionbaselineunionparnumérolignesmodifiés.
- RespecttravailCI/E2Eétranger,aucunworkflow/lanceurCIédité. Commit/pushdecenouveaulotnonlancéavantvalidationetcoordinationpublication.
