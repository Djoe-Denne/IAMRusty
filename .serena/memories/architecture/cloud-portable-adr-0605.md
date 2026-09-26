# ADR-0605 — gold path Kind J3

- Canon : `docs/adr/0605-gold-path-kind-j3-dns-attach.md` — Proposed / Implemented. Ne SuperSède pas 0601/0604/0009/0010.
- Guide : `docs/platform-local-gold-case-implementation-guide.md`.
- Gold path = Kind J3 HTTP 200 ; monolithe `aiforall-local` = seul Lazaret ; curl hôte ≠ preuve.
- DNS : Service ClusterIP même nom que le Pod ; URL `http://plugin-{digest}.apparatus-plugins.svc:8080`.
- **Écart 0604** : overlay démo reste ; Calico v3.29.7 enforce NP sur `aiforall-local` seulement, pas SuperSéde « Calico hors J3 ».
- **Schedule** : Cosign verify fail-closed (`envelopeReference` + `verify_cosign_signature`) avant create Pod. D-TRANSIT-TCB : OpenBao -dev local OK ; prod = Transit dédié, séparé du KV, seul SA admit-sign signe.
- Résiduel sécu S-1 : verify ne relit pas le payload enveloppe (swap cri_image + ref recyclée).

Voir le fichier ADR.
