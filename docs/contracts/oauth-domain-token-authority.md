# Autorité configurée du mint OAuth du domaine

2026-10-29 — auteur architecte/pool-4 ; **Accepted / Unimplemented**.
Le lead a lu intégralement les86lignes de la proposition finale et ratifie
explicitement ce contrat sous la délégation utilisateur, AVANT correction de
production ou migration du constructeur. Proposition originale conservée dans
le worktree auteur. Aucun changement d'ADR Accepted0304/0311/0314/0412 ou de
l'[autorité locale ratifiée](0412-oauth-proof-authority-alignment.md).

## Décision et alternatives

Conserver le mint intermédiaire et le contrat du domaine ; injecter issuer ET
audience explicitement depuis la configuration dans les DEUX constructions
production de TokenService. Réutiliser TokenClaims::new_with_issuer_audience et
le codec existant. Changement source interne des appelants migrés ensemble,
pas du protocole HTTP/JWT sérialisé, portencodeur, tuplecallback ou stockage.
Aucun constructeur production à deux arguments implicitementlegacy ni builder
permissif. Pas de nouveau getter sur JwtTokenEncoder.

Retrait du mint ignoré rejeté pour ce lot : changerait process_callback(User,
String,email), ses responsabilités/validation/effets, ou produirait un faux jeton
vide pour un utilisateur complet. Doublemint EXISTANT conservé, aucun nouveau
mint. Rejetés aussi : modifier DEFAULT_JWT_ISSUER globalement, fallbackiamrusty,
Host/Origin/redirect, autorité déduite d'une clé, relaxation du codec, autrealgorithme
ou provider. Le binding déjà ratifié est rétabli, pas une nouvelle autorité.

## Cause, références et preuve préalable

Briefing `.cursor/review-briefings/20261029-oauth-complete-user-issuer-conflict-3c2e7ea.md`.
TokenService::generate_token utilise actuellement TokenClaims::new/issueriamrusty,
alors que require_active_signing_epoch exige key.issuer==claims.iss==issuerURL
configuré. Mint domaine des utilisateurs complets seulement, après consommation
transaction et sauvegarde fournisseur. Son résultat ignoré par le usecase reste
bloquant avant vrai accessmint et session. 401 n'annule pas ces effets antérieurs.

Sources auteur : domain/service/token_service.rs8–57 ; entity/token.rs43–73 ;
oauth_service.rs143–160 ; application/usecase/oauth.rs232–289 ; setupapp544–564,
654–666,695–737,825–868,890–965,1348–1353. Deuxprod au setup, même codec configuré.
Audience disponible mais non relayée aux deps du domaine. Portencodeur sansgetter,
codec issuer/audience privés et fences inchangés.19appels connus du constructeur
dans le canon WIP :15inline token_service,1fixtureOAuth,2setup,1diagnosticcodec.
Nouvelappel positif configuré pourra devenir20e ; index/matching ne prouvent pas
absence universelle d'alias, compilation des consommateurs reste obligatoire.

Diagnostic parent réel préalable :
`cargo test --locked -p iam-infra --test rs256_jwks_roundtrip oauth_domain_mint_and_configured_access_mint_with_url_issuer -- --exact --nocapture`
1PASS/0ignoré/2filtrés,.03s/process61.739s, log60177b82… ; ancien mint refusé,
accès configuré accepté sur deux codecs RS256/registrePEM équivalents existants.
Ce PASS diagnostique n'est PAS correctionprod, login200 ou navigateur. Architecte
n'a exécuté aucun run ni inspecté ce journal ; attestationparent distincte.

## API et raccordement normatifs

1. Champs privés immuables `issuer: String`, `audience: String` dans TokenService.
   API exacte `pub fn new(token_encoder: Arc<dyn JwtTokenEncoder>, token_duration: Duration, issuer: String, audience: String) -> Self`.
   Stocker sans réécriture/alias/fallback. Pas de nouveau typepublic, Result,
   parseURLdomaine, surchargelegacy/getter. Configuration validée reste au root
   existant ; valeur incohérente jamais réparée depuis clé ou requête.
2. Signature generate_token inchangée. UNE construction
   `TokenClaims::new_with_issuer_audience(user_id, username, self.token_duration, &self.issuer, &self.audience)`
   et UN appel du même encodeur. MappingTokenGenerationFailed et validate_token
   inchangés ; pas retry ou seconde signature après refus.
3. Setup : ajouter `audience: String` à IamUsecasesDeps, depuis
   `config.jwt.audience.clone()` de la configuration déjà fournie au codec.
   Relayer à OauthLinkDeps ET ProviderUserTokenDeps, déstructurer/cloner localement.
   Réutiliser platform_issuer EXISTANT de IamRepos/deuxdeps, passer son clone et
   cette audience aux deux TokenService::new. Duréeexistante1h inchangée.
   Pas rechargement config, lecturecodec ou nouveau calcul URL.
4. Tous appelants migrés ensemble. Fixturesunitaires/mock/HSIsolatedTest peuvent
   fournir DEFAULT_JWT_ISSUER/AUDIENCE.to_owned() EXPLICITEMENT dans leur seul
   constructeur local ; pas fallback runtime, brancheproduction ou DEFAULT changé.
   Dans oauth_service.rs SEULE la fixture missing-client est adaptée.

Autorité locale8080/iam et redirects18080 inchangés. Audience exacte configurée,
défautplateforme aiforall inchangé, pas audience d'organisation ou issue du Host.

## Invariants et limites

RS256/kid/issuer/trust/organisation/version/provider/registry/signature, prépublication,
révocation/rotation/récupération atomiques, quotas/refus sans dommage et fence final
après await du signer inchangés. Fraîcheur maximale60s même en panne, étatvide sans
résurrection, pas cache ni PEMfallbackprod. Strings d'autorité ne sont pas un état
de clé autoritatif. Aucun droit accordé par ce seul issuer ; (iss,sub) conservé.
org=None pour ce chemin plateforme, aucun état/événement/FGA à migrer.

Utilisateur incomplet garde zéro mint domaine ; complet garde son mintdomaine
existant puis unique accessmint/refresh/session du usecase. JWTintermédiaire
jamais renvoyé en substitution si accèsfinalrefusé. Rotation entre les deux mints
peut encore refuser le second ; pas atomicitécrypto multiopérations promise ou
clé forcée. ConsommationOAuth/sauvegarde fournisseur restent dans leur ordre,
pas replay/échange répété/rollbackglobal inventé. Transport/PKCE/state/cookies/CSRF,
usagefinal/session/refresh et confidentialité inchangés. Aucun JWT/clé/code/state/
cookie complet dans sorties/logs/tests.

## Allowlist — cinq fichiers IAM

- domain/src/service/token_service.rs : champs/constructeur/mint et unitairesinline.
- domain/src/service/oauth_service.rs : constructeurfixtureStubEncoder/missing-client
  SEULEMENT, aucun corpsmétier/contratcallback.
- setup/src/app.rs : plumbingaudience et DEUX constructionsproduction.
- infra/tests/rs256_jwks_roundtrip.rs : diagnostic conservé et positif configuré.
- tests/oauth_browser_transactions.rs : second login après completion/harness existant.

Exclus : entity/token.rs, port/service.rs, jwt_encoder.rs, application/oauth.rs,
configuration, migrations, SDK/gitlink, FGA/remote, cookies/routes/TLS/Netpol/imagesbase,
nouveau harness/fichierprod ou couchecompatgénérique. Autreappelant prouvé nécessite
extension explicite lead AVANTédition. Migration des appelants Rust ensemble,
aucune conversion DB/config ni assemblage de constructeurs mixte.

## QA ciblée et gates

- `oauth_domain_mint_and_configured_access_mint_with_url_issuer` : exercer vrai
  TokenService CONFIGURÉ et vrai accèsRS256, vérifier signature/iss/aud/sub/absenceorg
  sans JWTdump. Conserver legacyrefusé avec DEFAULTs EXPLICITES seulementfixture.
  Ajouter positifdomaineURL distinct et négatifmismatchaudience à vérification ;
  fixturesregistres indépendantes, pas mockcodec/relaxationregistre pour verdir.
- `oauth_registration_completion_then_second_login_uses_configured_authority` :
  login202→completionusername→NOUVELLE transaction/cookie/state même utilisateur,
  callbacksuccèsréel/accessvalide/refreshsessionpersistés. ConnecteurHTTPfixture
  existant, échanges comptés ; replayconsommé refusé sans nouveléchange. Garder
  registration→refresh précédent, pas le renommer pour masquer le trou.
- Unitairesinline : issuer/audience explicites incluantaudiencenondefault,
  durée/sub/username/org/jti et UNencode préservés, erreursTokenGenerationFailed.
  Nouveau nom distinct à inscrire avant run si nécessaire.
- Cibles : iam-domain --lib service::token_service et missingclientOAuth ; iam-infra
  rs256_jwks_roundtrip ; iam-service oauth_browser_transactions testnommé. Compile
  des deux constructions via iam-service/iam-setup, fmt/check proportionnés.
  Windowsnatifs24jobs/E:/unCargo, fixturesITexistantes/registre/cleanup. Aucun run
  auteur, pas de nouvelle campagne/harness par principe.

Après correction intégrée et IT finales, vrai navigateur0412 via Envoy dans
laboratoire ratifié reste requis. Testcodec/IT ne le remplace pas. Aucun PASS
global ou production depuis un succèslocal. Contrat sourceinterne crossmodule
seulement ; aucune ADR nouvelle car applique le binding déjà Accepted.

## Amendement QA — sélection du binaire OAuth existant

Le lead a lu intégralement les20lignes finales Proposed de l'architecte, puis
ratifie explicitement ce DELTA AVANT édition OPS. Ratification distincte du
contrat principal ; aucun choix core/API ou ADR rouvert.

Deux fichiers OPS autorisés pour ce seul complément :
- ops/tests/it-native.ps1 : Resolve-ItTestBinary et message d'erreur seulement ;
- ops/tests/test-it-native-selection.ps1 : oracles AST/fakes existants.

Ajouter UNIQUEMENT le couple sensible à la casse Suites=["iam"] (une occurrence)
et Binary="oauth_browser_transactions". Pas trim/alias/glob/nouveau paramètre,
filtreTestName/packageoverride/argvlibre. Deux couples Manifesto et binairevide
gardent leur comportement. Autres suites, couplescroisés, plusieurs suites même
identiques, casse/espaces/nom/chemin/option/injection contradictoires refusés AVANT
Get-Command cargo/docker/observeur/runtime. Aucun repliIAM--tests sur refus.

BrancheCargo existante, mapping iam-service, threads/jobs, fixtures, observeur,
registrecréationcontemporain/IDscomplets/cleanupfinally restent INCHANGÉS. Pas
nouveau wrapper/module/fixture ou permission. L'extension choisit une cible déjà
supportée ; toutIAM serait disproportionné, Cargo seul sans registre/cleanup ou
copie d'un ancienwrapper ne constitue pas une alternative plus petite conforme.

Gate avant live : `powershell.exe -NoProfile -File ops/tests/test-it-native-selection.ps1`.
ASTpur et spyCargo : positifsIAM/Manifesto/vide, négatifscroisés/casse/espaces/
multisuites/répétition/chemins/options/injection, placementavantprobe ; vraie
branche extrait exactement `test --locked -p iam-service --test oauth_browser_transactions`
sans filtre et anciensargv inchangés. Aucun Cargo/Docker réel dans ce gate.

Après correctif intégré/revu, gate et préflight/bail :
`powershell.exe -NoProfile -File ops/tests/it-native.ps1 -Suite iam -TestBinary oauth_browser_transactions`.
Binaireentier (neuf tests selon lecture source), pas toutIAM. Windowsnatifs24jobs/E:/
unCargo, fixturesexistantes/registre et cleanup immédiat mêmeéchec ; pasKind.
Échec environnement/cleanup reste telquel, pasprune/force/reclassementPASS.
NavigateurréelEnvoy distinct et ultérieur ; auteur n'a exécuté aucun run.
