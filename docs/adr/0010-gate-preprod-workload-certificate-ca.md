# ADR-0010 : La modalité d’émission de certificat workload et la CA logicielle actuelle sont acceptables hors production et bloquantes avant prod

- Statut : Proposed
- Réalité : Unimplemented
- Date : 2026-09-25
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : P3 (complète 0007 ; pré-production après P3 Implemented)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0007](0007-apparatus-p3-capability-boundary-after-accept.md), [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md)

`Accepted` ratifierait la **gate** ci-dessous. `Réalité : Unimplemented` : la cible (modalité d’émission + autorité CA adressées pour la prod) **n’est pas** dans le dépôt. Le code P3 actuel (CA logicielle `platform-internal-ca`, generate-if-absent, CSR-from-workload par défaut T3–T13) reste la **dette acceptée hors prod** — il ne satisfait pas cette ADR pour la production. Cette ADR **ne SuperSède pas** [0007](0007-apparatus-p3-capability-boundary-after-accept.md) (`Réalité : Implemented` inchangée).

## Contexte

[0007](0007-apparatus-p3-capability-boundary-after-accept.md) livre la frontière P3 (BC Lazaret, identité hybride, CA interne plateforme V1). Les humains qui croient « P3 fini / prêt prod » lisent 0007 Implemented. Or 0007 laisse **Non décidé** :

- produit ou bibliothèque CA (aucun nom figé) ; TTL et rotation numériques exacts ;
- modalités d’émission : CSR depuis le workload vs keypair injecté.

T3–T13 enregistrent des **défauts d’implémentation** (CSR-from-workload, nom logiciel `platform-internal-ca`, generate-if-absent, certificats / TTL numériques) qui **ne** sont **pas** des chiffres Accepted. Sans gate écrite, ces défauts laptop/kind pourraient être shippés en prod par omission. [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md) est une **sœur** gate pré-prod (scale / isolation instance) — **zéro** certificat / CSR / CA / PKI.

## Décision

1. **Limite actuelle acceptable hors prod.** CSR-from-workload vs keypair injecté **reste ouvert**. CA logicielle actuelle (`platform-internal-ca` / generate-if-absent / certificat quelconque T3–T13) : **acceptable** pour laptop / kind / environnements hors production.
2. **Gate pré-prod bloquante.** La modalité d’émission **et** l’autorité CA **doivent** être adressées **avant** toute mise en production. Interdit de présenter 0007 Implemented comme « prêt prod » sur ce point.
3. **Mécanisme Non décidé.** CSR-from-workload vs keypair injecté ; produit / bibliothèque CA ; PKI cloud vs interne ; TTL / rotation chiffrés = **Non décidé** ici ; à trancher avant prod, **pas** dans cette ADR.
4. **Ne SuperSède pas 0007.** `Réalité : Implemented` de 0007 reste inchangée ; cette ADR marque uniquement la **dette pré-prod** visible.

## Conséquences

- 0007 reste Implemented pour P3 ; cette ADR marque la **dette pré-prod** visible sur certificat / CA.
- Accept de cette ADR = obligation de traiter modalité d’émission + autorité CA avant prod ; **pas** un contrat d’implémentation ni un choix CSR / keypair / produit CA.
- Toute roadmap « ship prod » doit citer 0010 comme gate ouverte tant que `Réalité` ≠ Implemented.
- Convention d’endpoint (URL fixe, port, id dans le path, gRPC, etc.) : **Non décidé / hors périmètre** de cette ADR.

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| SuperSéder 0007 / rétrograder sa Réalité | P3 est livré ; la gate est distincte |
| Figé CSR-from-workload ou keypair injecté maintenant | Modalités déjà Non décidé 0007 ; hors périmètre |
| Figé produit CA / TTL / rotation / PKI cloud vs interne | Mécanisme hors périmètre ; à décider avant prod |
| Reporter la gate à une note wiki / closeout seul | Les lecteurs de 0007 Implemented ne verraient pas le blocage |
| Traiter 0007-closeout D-CA comme gate « ne pas shipper prod » | Dette d’inventaire, pas une gate bloquante écrite |

## Non décidé ici

- CSR depuis le workload vs keypair injecté (renvoie [0007](0007-apparatus-p3-capability-boundary-after-accept.md))
- Produit ou bibliothèque CA ; PKI cloud vs interne ; TTL et rotation numériques
- Convention d’endpoint (URL fixe, port 8080, id dans le path, gRPC, etc.) — **Non décidé / hors périmètre**
- Scale on demand / isolation d’instance (renvoie [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md))

## Références

- Canon voisin : [0007](0007-apparatus-p3-capability-boundary-after-accept.md) ; sœur gate [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md) ; closeout historique [0007-closeout.md](0007-closeout.md) (**non** modifié par cette ADR)
- Code (dette hors prod, preuve de la limite) : défauts T3–T13 Lazaret (`platform-internal-ca`, generate-if-absent, CSR-from-workload)
- Preuve d’implémentation de **cette** cible : **aucune** (`Réalité : Unimplemented`)
