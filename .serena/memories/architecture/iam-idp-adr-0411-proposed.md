Jalon : IAM-IdP (hors Apparatus). Canon : `docs/adr/0411-idp-provider-slug-registry-fail-closed.md`. Statut : Proposed. Réalité : Unimplemented.

- `Provider` = newtype slug (`^[a-z]+$`, case-fold) ; catalogue = `[[idp.connectors]]` au boot ; `HashMap<Provider,_>`.
- N+1 IdP = copie `GitHubConnect/` + compose + ligne registry + restart IAM. Hot-load / wasm / SDK in-process : non.
- Admission = ligne complète (HMAC ≥16, `base_url`, Callback+Relink). Skip silencieux interdit. 400 syntaxe / 422 hors registry. 0409 inchangé.
- SuperSède : aucune (gradue leftover 0407 §8 / 0408 long terme). Pas Accepted.
- Voir le fichier ADR.