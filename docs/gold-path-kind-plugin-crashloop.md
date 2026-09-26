# Note — CrashLoop gold path Kind (`plugin-98ba747f…`)

**Pas une ADR.** Marqueur d’incident, à part des décisions Cosign / registre / NetworkPolicy. Gold path : [ADR-0605](adr/0605-gold-path-kind-j3-dns-attach.md). PKI workload (lien possible, non diagnostiqué) : [ADR-0010](adr/0010-gate-preprod-workload-certificate-ca.md).

Observé le 2026-09-26 sur `kind` `aiforall-local`, namespace `apparatus-plugins`.

## Symptôme

- Pod `plugin-98ba747fc572de29d76dfd08f9253778` : `CrashLoopBackOff` (plusieurs redémarrages).
- Service leftover `apparatus-reference-kv` (ClusterIP) encore présent à côté du Service nominal `plugin-98ba747f…` (0605 = Service même nom que le Pod).
- Preuve gold antérieure : invoke in-cluster 200 + hostname de ce pod ; le CrashLoop est **postérieur** à cette preuve, pas un verdict 0605.

## Ce qu’on sait

- Digest = pin catalogue `sha256:98ba747f…` (32 premiers hex). Image CRI reference-kv.
- Leftover `apparatus-reference-kv` = dette 0604 (hop URL manuel), hors kustomization nominale, hors `just prove-gold`.
- Le source operator (après revue 0605) recrée le Pod si le label `app.kubernetes.io/instance` manque — **image controller Kind non rebuild** au moment de la note ; ce correctif n’explique pas à lui seul le CrashLoop déjà vu.

## Ce qu’on n’a pas diagnostiqué

- Logs conteneur / raison `LastState` (enroll 4xx/409, CA vide, crash binaire, secret OpenBao).
- Lien causal avec emptyDir CA / rotation (hypothèse [0010](adr/0010-gate-preprod-workload-certificate-ca.md) : CSR / `platform-internal-ca` generate-if-absent — **non confirmé**).
- Si le leftover Service participe au crash (le locator DNS 0605 vise `plugin-{32hex}`, pas `apparatus-reference-kv`).

## Hors cette note

Ne pas mélanger avec : lecture anonyme zot/catalogue ; enforcement NetworkPolicy ; durcissement Transit/Cosign prod.
