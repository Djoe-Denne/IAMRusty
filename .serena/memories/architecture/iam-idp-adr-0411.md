Jalon : IAM-IdP (hors Apparatus). Canon : `docs/adr/0411-idp-provider-slug-registry-fail-closed.md`. Statut : Accepted. Réalité : Implemented.

- `Provider` = newtype slug (`^[a-z]+$`, case-fold) ; catalogue = `[[idp.connectors]]` au boot ; `HashMap<Provider,_>`.
- N+1 IdP = copie `services/GitHubConnect/` + compose + ligne registry + restart IAM. Hot-load / wasm / SDK in-process : non.
- Admission = ligne complète (HMAC ≥16, `base_url`, Callback+Relink). Skip silencieux interdit. 400 syntaxe / 422 hors registry. 0409 inchangé.
- SuperSède : aucune (gradue leftover 0407 §8 / 0408 long terme). Accepted+Implemented 2026-09-22 (GH/GL). Pas de service HF.
- Voir le fichier ADR.
- Actualisation 2026-11-06 : onboarding N+1 par nouveau service partiellement supersédé par ADR0413 Accepted/Unimplemented ; cible entrée d'instance compatible dans OAuthConnect + registry IAM. Slugs/fail-closed/400-422/no-hot-load demeurent ; pas de confusion entre type de profil, instance logique et replica/cluster.
