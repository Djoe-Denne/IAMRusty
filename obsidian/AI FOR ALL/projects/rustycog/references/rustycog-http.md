---
title: RustyCog HTTP — propriétaire et arrêt observé
category: references
tags: [reference, api, rust, architecture]
sources:
  - _sources/opencode-telegraph-outbox-20261027/inventory.json
  - docs/contracts/http-runtime-owned-shutdown.md
summary: "API HTTP additive possédée : adresses réelles, shutdown sous deadline et jointure des descendants. Couverture ciblée port0/CONNECT ; Drop ne vaut pas async join."
provenance:
  extracted: 0.96
  inferred: 0.04
  ambiguous: 0.00
created: 2026-10-27T19:27:11Z
updated: 2026-10-27T19:27:11Z
---

# RustyCog HTTP : propriétaire et arrêt observé

Référence restaurée depuis la mission Telegraph : cette page couvre le cycle de vie HTTP possédé et ses limites, pas toute l'API de routage, TLS ou permission du framework. Voir [[projects/telegraph/references/telegraph-http-and-notification-api]].

## Pourquoi le parent ne suffit pas

La fin du futur serveur ou son annulation ne prouvent pas la destruction des connexions, handlers et descendants. Un signal d'arrêt, un compteur ou un `Drop` ne remplacent pas la jointure observée.

## Surface additive livrée

- `start_router_owned(router, config)` rend un `OwnedHttpServer` retenu par le caller.
- `local_addrs()` rend les adresses réellement liées, dont le port attribué pour port 0.
- `wait_for_server_exit()` observe le serveur/coordinateur, pas l'achèvement exhaustif.
- `request_shutdown(HttpShutdownBudget)` transmet grâce et deadline absolues ; les répétitions ne rallongent pas les bornes.
- `join()` atteste le travail effectivement terminé et joint. Annuler un futur de jointure emprunté ne doit pas perdre les moyens d'observer les descendants.
- Legacy `serve_router` reste disponible ; abandonner cette ancienne API n'acquiert pas rétroactivement la nouvelle garantie.

Dans Telegraph standalone et le listener partagé du monolithe, les helpers composent parents métier et owner HTTP sous la même grâce à t0+5 s et jointure/arrêt absolus à t0+6 s, stop inclus. Les autres standalones ne sont pas migrés implicitement.

## Compatibilité, erreurs et limites

Port 0 et Extended CONNECT HTTP/2 ont des témoins rouges puis verts et revues ciblées. La preuve CONNECT couvre négociation/routage, pas le lifetime de tout upgrade arbitraire. Les résultats loopback et protocoles ciblés ne valent pas validation globale de la plateforme.

`Drop` demande l'annulation sans attendre : panic arbitraire ou annulation extérieure du futur complet ne deviennent pas une attestation d'async join. La revue statique du raccordement de test vérifie les chemins de timeout, sans prétendre les avoir tous forcés en runtime.

Le SDK exact d60 est publié et piné dans le commit application39024b2 : [[projects/aiforall/references/mission-telegraph-outbox-20261027]]. Le statut ADR0503 reste Proposed, sans promotion automatique par cette référence.

## Related

- [[projects/aiforall/skills/proving-leased-outbox-recovery]] — garder les handles disponibles autour d'une reconstruction bornée.
- [[projects/telegraph/references/telegraph-service]] — adoption par le composition root.
- [[projects/rustycog/references/rustycog-outbox]] — arrêt local distinct du claim SQL durable.
