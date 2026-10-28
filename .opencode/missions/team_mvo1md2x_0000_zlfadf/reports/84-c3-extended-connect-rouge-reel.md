# Rapport 84 — C-3 Extended CONNECT rouge réel

## Exécution recoupée

pool12 a pris le slot Cargo24 après préflight zéro Cargo. Commande réelle : `cargo test --locked --manifest-path rustycog/Cargo.toml --features http --lib owned_http2_announces_and_routes_extended_connect -- --nocapture`. Compilation réussie, exit101,0PASS/1FAILED/0ignored/45filtered,0.01s.

Lead lit log/JSON `.tmp/http-compat-evidence/c3-functional-red.*` via Context Mode : sourceSHA7a1930f78dd42a18ffdb91e45d527b4d195993d09d9296be7b2722318fb65489, logSHA4f82d74a435d48dc268ef6a0a8571c00aad9ef1e91d9e0c3cf20d3d83998ad04. Marqueur : probe_h2=true, setting=false, routed/status/marker=None ; owner joinOkforcedfalse, clientdriverOkOk ; cleanuptrue **avant assertion du setting**.

Corps1143–1245 relu : vraie connexion HyperHTTP2, probe comme barrière SETTINGS, observation via Connection::is_extended_connect_protocol_enabled, vraie requête CONNECT avec extensionProtocol, router enregistre méthode/version/protocole et marqueur. Réponses/client libérés, owner joint, clientdriver aborté puis joint avant assertions. Le GET n'est pas utilisé comme substitut à CONNECT. Ce rouge établit la régression de négociation ; l'oracle de routage est conservé pour le vert.

Premières erreurs de méthode API du test et run filtreexact0tests sont conservés mais **pas preuves rouges fonctionnelles**. Aucun fixproduction, dépendance/feature/harness nouveau ou runtimeDocker/IT créé. Les13preuvesC1 ne sont pas rejouées.

## Suite

Auteur tests finalise checkscompilation des cibles fixture sans exécution IT, commits tests-only et provenance, puis libère slot. Lead gèlera/intégrera cette remise avant correctionC3. C2 reste non exécuté pour prérequis extractionproduction explicitement validé au rapport83, pas une nouvelle demande humaine. Oracles RunningApp0/2+HTTPjoignable doivent être conservés ; leur compilation ne devient pas preuveITAC4/5.

Aucune publication/pin final ou clôture C-2/C-3. Correctif du setting reste dans le contrat de préservation de protocole, sans nouveau backend/API ; tests immuables après intégration avant auteurproduction.
