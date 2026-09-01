# Parcours identité (IAMRusty)

IAM est le seul émetteur de JWT et le compte utilisateur unique. Les scénarios exécutables vivent dans [`IAMRusty/qa/scenarii/`](../../IAMRusty/qa/scenarii/) — cette page les indexe.

Préfixe : `/iam`. Compose : port 8080.

## Compte

| # | Parcours | Fichier |
|---|---|---|
| 01–02 | Création via GitHub OAuth (succès / échec) | `01_…` / `02_…` |
| 03–04 | Création via GitLab OAuth | `03_…` / `04_…` |
| 05–06 | Création email + mot de passe | `05_…` / `06_…` |
| 18 | Disponibilité du username | `18_username_availability_check.md` |
| 19 | Vérification d’email | `19_email_verification_success.md` |
| 25 | Profil (`GET /api/me`) | `25_user_profile_management.md` |

Signup email : ne **rattache pas** un mot de passe à un compte déjà existant (contrat red-team P0). Vérification mail → événement `user_email_verified` → Telegraph (notification).

## Liaison de providers

| # | Parcours |
|---|---|
| 07–08 | Lier GitLab ↔ GitHub |
| 09 | Ajouter un mot de passe à un compte OAuth |
| 10 | Ajouter un OAuth à un compte password |
| 11 | Login OAuth sur user existant |
| 22–23 | Relink + conflits |

OAuth `state` : HMAC + TTL + bind `user_id` sur les flux authentifiés (P0). Fusion email seulement si l’IdP affirme un email **vérifié**.

## Session

| # | Parcours | Notes |
|---|---|---|
| 12–13 | Login email / password | Rate limit sur les routes auth |
| 14–15 | Refresh | Rotation ; détection de course |
| 16–17 | Reset MDP (anonyme / authentifié) | Révoque les sessions au reset (P1) |
| 24 | Validation JWT | Claims + algo figé |

Access token ~15 min (`expiration_seconds = 900`). Refresh ~30 j, stocké hashé.

## Tokens provider (interne)

| # | Parcours |
|---|---|
| 20 | Récupération token IdP (route interne authentifiée) |
| 21 | Révocation |

Ces routes restent authentifiées (`/internal/{provider}/token`). Ne pas les exposer comme API publique.

## Résilience / sécurité

- `26_security_edge_cases.md`
- `27_complete_multi_provider_workflow.md`
- `28_api_resilience_testing.md`

## Événements émis vers Telegraph

`user_signed_up`, `password_reset_requested` → email ; `user_email_verified` → notification in-app. Voir [notifications.md](notifications.md).

## API (rappel)

Publiques : signup, login, verify, reset, OAuth start/callback, username check, refresh, JWKS.

Authentifiées : `/api/me`, reset authentifié, link/relink provider, routes internes token.
