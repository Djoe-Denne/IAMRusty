# Avis architecte : isolation effective Windows non établie

Source native : result_mvvd4ub4_001d_dm7qv1, architecte12, même session que le
contrat Windows. Avis documentaire, aucune correction ni exécution vérifiée ici.

## Décision

**Le témoin actuel ne suffit pas à qualifier le sandbox.** Recommandation :
**Chromium INCONCLUSIVE**, même si version et JS réussissent, tant que la restriction
effective liée au cas n'est pas établie. Cela ne signifie **ni sandbox désactivé
ni évasion démontrée**. Cet avis ne lève pas lui-même le gel de lancement.

## Preuves et sources

- [Chromium156 sandbox.md:554–599](https://github.com/chromium/chromium/blob/156.0.8078.73/docs/design/sandbox.md)
  confirme primaire lockdown et impersonation initiale plus puissante. Un primaire
  restrictif peut être observé avant l'entrée effective dans l'état final.
- [target_services.cc:127–155](https://github.com/chromium/chromium/blob/156.0.8078.73/sandbox/win/src/target_services.cc)
  effectue notamment l'abaissement d'intégrité, RevertToSelf, le retrait de handles
  et les mitigations différées. Des échecs critiques entraînent une terminaison.
- [renderer_main.cc:295–342](https://github.com/chromium/chromium/blob/156.0.8078.73/content/renderer/renderer_main.cc)
  active normalement le sandbox avant la boucle ; le
  [delegateWindows:72–88](https://github.com/chromium/chromium/blob/156.0.8078.73/content/renderer/renderer_main_platform_delegate_win.cc)
  appelle LowerToken. Corroboration du fonctionnement attendu, pas liaison runtime
  entre le jeton échantillonné et le renderer du cas.
- [sandbox.md:136–157](https://github.com/chromium/chromium/blob/156.0.8078.73/docs/design/sandbox.md)
  décrit la forme lockdown la plus restrictive : groupes ordinaires deny-only,
  SID restreint nul et aucun privilège. Aucun privilège activé n'établit pas aucun
  privilège présent ; ne pas transposer cette forme à toute variante AppContainer.
- Microsoft distingue [jeton primaire et impersonation](https://learn.microsoft.com/en-us/windows/win32/secauthz/impersonation-tokens).
  Les [jetons restreints](https://learn.microsoft.com/en-us/windows/win32/secauthz/restricted-tokens)
  combinent groupes, deny-only, privilèges et SIDs : un indicateur ne décrit pas
  toute la politique.

## Preuve minimale accessible dans le mandat

Pas établie avec les observations actuellement admises. Il manque trois liens :
identité du renderer participant au cas ; moment après bootstrap ; politique
effective complète tenant compte de l'impersonation. Ni un délai750ms, ni JS42,
ni le Job Object ne les remplacent.

[OpenThreadToken](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openthreadtoken)
puis [GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation)
avec TOKEN_QUERY pourraient contribuer à une enquête future sous les accès DACL
accordés. OpenAsSelf choisit le contexte du contrôle, pas une élévation. Mais cet
ajout seul ne fermerait pas R-2 : threads pertinents, accès refusés inconnus et
corrélation au cas restent nécessaires. Une photographie de threads ne vaut pas
attestation complète. Cette observation supplémentaire n'est pas implicitement
autorisée ; aucune instrumentation n'est proposée pour cette tranche.

## Cas borné non qualifiant

Après fermeture vérifiée de S-1/R-1 et disposition explicite de la revue,
**R-2 ne démontre aucune raison de sécurité supplémentaire interdisant en soi
l'unique cas déjà autorisé** : code vérifié, sandbox maintenu, contexte neuf,
about:blank/42, sans contenu applicatif ni secret.

Conditions : aucune voie ChromiumPASS depuis le booléen sticky ; préconditions,
confidentialité, propriété des processus, jonctions et retraits inchangés.
Distinguer JSréussi, versionétablie, restrictionsprimairesobservées, isolation
effective non prouvée et cleanup selon ses propres preuves. Pas de nouveau GO de
jalon ni relaxation déduits de cet avis.

## Oracles et limites

Ne suffisent jamais à PASS : primairelockdown avec impersonation initiale plus
forte ; zéro privilège activé mais privilèges présents ; autre renderer seulement ;
lecture refusée ; identité/moment non liés ; délai écoulé.

Localisation : R-2 snapshot68631c0, windows-native.cs:91–104,272–278 et revue
indépendante persistée. Impact : qualification/provenance du témoin, pas réouverture
de S-1/R-1. Les corrections en cours n'ont pas été vérifiées par cet avis.
Pas de nouvelle ADR/contrat : le contratWindowsAccepted impose déjà INCONCLUSIVE
si le témoin manque. Aucune édition, commandeWindows, sonde, test ou runtime ;
gitdiffcheck non exécuté.

## Disposition du lead

Avis retenu dans le mandat existant : pas de nouveau mécanisme d'observation ou
preuve intrusive pour cette tranche. R-2 reste un **gap de qualification explicite**,
pas une acceptation d'un faux PASS. Le parent conserve ChromiumINCONCLUSIVE et un
exit non nul même si APIversion/checks réussissent. Les versions/outils/cleanup
gardent leur portée propre et ne deviennent pas un succès global.

Lancement toujours gelé jusqu'à correction intégrée de S-1/R-1 et relecture ciblée
des trois dispositions. Une fois ces conditions fermées, le lead pourra exécuter
le seul cas non qualifiant déjà autorisé, sans nouveau navigateur/canal/scénario.
