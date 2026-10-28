# Rapport 91 — C-3 causal vert, C-2 rouge, contre-épreuve prête

## Résultats réels recoupés

Lead lit logs/JSON `.tmp/http-followup-evidence/` :
- C3premièrexécutioncandidate :1PASS/0ignored/45filtered/exit0, freshAfterProbe=Ok(true), oldAtomic=false, freshAfterCONNECT=Ok(true), vraieCONNECT/H2/Protocolrustycog-test routée200+markerattendu, ownerdrivercleanuptrue. **Atomicstale vs gettervivant désormais discriminé dynamiquement.**
- C3sourcefinale après retrait de la télémétrie transitoire : même1PASS/0ignored/exit0, freshAfterProbe=true et CONNECTcomplet, cleanupavantassert. L'oracle n'a pas été réduit à route200 ; settingtrue toujours requis.
- C2 : `runtime::tests::shared_startup_port_zero_serves_health_on_actual_port` compile puis0PASS/1FAILED/0ignored/7filtered/exit101. expected55827/bound55828 ; healthBOUND200+markerprouve vrai startup, healthEXPECTEDtimeout1s, shutdownjointOkforcedfalse avantassert bound!=expected. Source202–259 relue : vrai helperapp/host, pas copie de normalisation ni faux chemin.

## Remise tests figée

SDK tests **d60a88b2f2f499768a807941a1b4c80c9a366b3f**, parent5eaa, tree8bba2cf45e4943eda72f2cc127e785973d3bbcff, clean. Productionprefix égale5eaa. SourceSHAf21b0f9ef43828ab33618598918b2bc0b5d47c0c6000d9f61a72c29532b375b4 ; suffixetestSHA5bffb931f47bcce1be7cb5df9c6d0799ccf0ae9295e4106f0c88f7b4b3cc80b9. FmtSDK0/nonstrictClippy0 recoupés.

AItestC2 seulement +59lignes ; hook206unefois, puis leadcommitnormalcourt **d36eac687903882e85b91e75a17bd7d03599d8ff**, treec05b27c4911e09c7d2f17fc2ed3ad30785eb44f1 exactindex, hookPASS. BrancheauteurCASactualisée sansreset/stash/bypass.17entrées hash-vérifiées archivées `.tmp/http-followup-delivery-evidence/manifest.json`. Pas encore native merge au lead, pas de productionC2corrigée.

## Contre-épreuve C-3 contrôlée par le lead

Avant nouveau run, lead crée SDKworktree secondaire sous pool12 `.tmp/http-c3-baseline-sdk` à partir de d60, puis remet uniquement les deux lignesproductionà3333 via patch dédié. Vérifications : productionprefixexact3333 ; testsbyte-identiquesd60 et candidate (suffixSHA5bff...), aucun autre fichierdiffvs3333. BaselinesourceSHAf4fed16d71867caf636f9e3489a787df29f4a5b7eaeaefb2b66b574a2c5d0ff2. Snapshotsparent, pas prodeditparauthorTests ou nouveauharness.

Preuveinput `.tmp/http-followup-evidence/baseline-input-lead.json`. Même auteurtest reçoit GO pour **un seul test exact** depuis rootAIpool12 avec --manifest-path .tmp/http-c3-baseline-sdk/Cargo.toml ; configCargo24/targetE, préflight+bail, aucune source à modifier. Résultat attendu discriminant mais non imposé ; compil/env/timeoutobservateur distingués, cleanupavantassert. Baselinecorrectement décritproduction3333+testsd60, pas HEADd60 utilisé comme faussepreuveproduction.

Contre-épreuve non encore exécutée à ce rapport, donc C-3 pasclosavantretour/revue. C2resteROUGE, fixnormalisation àlibérer APRÈStestsintégrés. PasDockerIT/13rerunglobal/push/pinfinal.
