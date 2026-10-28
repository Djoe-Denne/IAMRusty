# Rapport 88 — CONNECT routé, observation du setting à diagnostiquer

## Premier essai du correctif recoupé

pool13 confirme sourceSDK3333/AIe8987 propres avant delta. Diff productionSDK exact : builder devient mutable et http2().enable_connect_protocol() ; aucun test modifié. AIextractionruntime+deuxcallers préparée, sans normalisationport. Un seul essai substantiel.

Lead lit UTF16LE les logs locaux `.tmp/http-compat-fix-evidence/` : C3 compile puis exit101/0PASS1FAILED0ignored45filtered, **probe_h2=true, setting=false MAIS routed=Some(CONNECT,HTTP/2.0,Protocolrustycog-test), status200/markerattendu**, owner/clientcleanup avantassert. H2admis existant1PASS/0ignored/exit0. Aucun C3vert ou acceptationprotocole globale revendiqué. Pas nouveau runtimeDockerIT ; slotCargo auteurRELEASED.

Ce n'est plus l'observation de baseline3333 (CONNECT non routé). Le rétablissement du routing apporte un progrès vérifiable ; pas deux corrections échouées sans progrès. Le test continue toutefois d'échouer à son assertionsetting, résultat non masqué. Hypothèse : l'atomic est échantillonné après Future::pollConnection puis lu après réception du probe, sans garantie de fraîcheur. **Hypothèse seulement**, getterHyper/notification à examiner avant décision.

## Gel et diagnostic indépendant ciblé

Productioncandidate et tests restent gelés. Cinq fichiers source/logs/exits copiés et hashes vérifiés avec patchdiff+manifest dans `.tmp/http-c3-setting-observation-failure/`, et copie identique sous pool12 `.tmp/http-c3-observation-review/`. Même auteur du test pool12 réactivé pour diagnostic lectureseule, sans Cargo/mutation à cette étape. Pas de copie du contexte auteurproduction, seulement source/diff/preuves.

Demande bornée : établir la cause getter vs atomicpérimé, proposer synchronisation observable (pas délaiartificiel), garder obligatoirement SETTINGS_ENABLE_CONNECT_PROTOCOL=true ET vraie CONNECTméthode/version/protocole/réponse etcleanup. Pas suppression du setting ni substitution par route200seule, dépendance/feature/harness nouveau ou oracleaffaibli.

Correctifproduction non intégré/committé comme vert, taskprodencours, helperC2 pasencorelivréintégré. Pas nouveaux tests exécutés parlead, pas rerunaveugle/13tests. Publication/pin et C2/C3 toujours ouverts. Après diagnostic seulement, décider l'expérience discriminante et son ownership ; si seconde tentative sans progrès, règle d'escalade maintenue.
