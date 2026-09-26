# Guide cas classique local (gold path)

**Fichier** : `docs/platform-local-gold-case-implementation-guide.md` (contrat d'éxécution, PAS une ADR).
**ADR qui fige le gold path** : `docs/adr/0605-gold-path-kind-j3-dns-attach.md` — Proposed / Unimplemented. Ne SuperSède pas 0601/0604/0009/0010.

Décisions 0605 :
1. Gold path = Kind J3 HTTP 200 ; monolithe `aiforall-local` = seul Lazaret ; curl hôte ≠ preuve.
2. DNS : operator Service ClusterIP même nom que le Pod ; URL `http://plugin-{digest}.apparatus-plugins.svc:8080` ; op dans le corps ; pas client kube Lazaret ; digest-DNS Kind only (prod → 0009).
3. Attach : writer managed remplit digest + declared_capabilities ; consents T5 ; pas seed / nouvelle route.

Ouvert : CSR/CA → 0010 ; scale/HPA → 0009.
