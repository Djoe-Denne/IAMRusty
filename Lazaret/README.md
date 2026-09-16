# Lazaret

Frontière de capacités (BC distinct de Manifesto). Slice T2 : health / ready uniquement.

- Préfixe HTTP : `/lazaret` (compose : port hôte **8084**)
- JWT consommateur : `[auth.jwt]` — `hs256_secret`, `issuer = "iamrusty"`, `audience = "aiforall"`
- AuthZ : pas de type OpenFGA ce slice (`InMemoryPermissionChecker`)
- Base : `lazaret_dev`

## Lancer

```bash
cargo run -p lazaret-migration -- up
cargo run -p lazaret-service
```

Config : `Lazaret/config/` (`LAZARET_*`).

## Documentation

- Handbook : [`docs/services/lazaret.md`](../docs/services/lazaret.md)
- JWT : [`docs/guides/jwt-consommateur.md`](../docs/guides/jwt-consommateur.md)
