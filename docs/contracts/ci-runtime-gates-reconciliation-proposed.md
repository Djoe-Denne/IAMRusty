# Réconciliation des oracles lexicaux CI runtime

- Statut : **Proposed** ; note de maintenance des tests, pas une nouvelle ADR.
- Analyse architecte du 12 novembre 2026, source `a2c0f80`, échec CI `43278101994`
  à `4fa7cfa`. Restitution lecture seule persistée par le lead.
- Aucun changement de Statut/Réalité d'ADR ou de contrat ratifié.

## Autorité existante et contradiction

Le [contrat runtime](apparatus-knative-runtime.md), §11/§14-T, impose de garder
les gates et leur périmètre. Ses annexes ratifiées B1/B3/B4 et §§13.1–13.5
ajoutent pourtant des données/ports de contrôle qui contiennent les lexèmes
interdits globalement dans certains anciens tests P1/P2/P3/P4.

La correction proposée ne change ni racine de scan, ni targets, ni tokens
interdits : reconnaître uniquement des identifiants/fragments contextuels fermés
du contrat existant. Aucun module complet nouveau n'est exempté. Une occurrence
interdite ajoutée ailleurs, dans le même fichier, ou sur la même ligne doit
rester détectée. Les exemptions préexistantes restent distinctes.

## Occurrences justifiées

- **P1/P2 :** types `RuntimeWriterLease*`, route `writer-lease` et colonnes de
  bail : B3/§13.1. `release`/`BlockRelease` ne sont pas des baux additionnels.
  Les deux commentaires « fencing »/« verifier factory » n'ajoutent pas de
  worker ou de Factory produit.
- **P3/P4 Manifesto :** champs DTO d'effet et permission `runtime.invoke` dans
  `runtime_control/begin.rs` et `maintenance.rs` : B1/B4/§13.2-D. Vérifier et
  réserver un effet SQL n'exécute pas son I/O. Le trait ApparatusRuntime reste
  sans méthode d'invoke.
- **P3 identité :** rôle fermé `RuntimeServiceRoleV1::Lazaret` et garde de rôle
  `require_lazaret` : §13.1-B/§13.3. Cela n'autorise ni client Manifesto→Lazaret,
  ni dépendance du BC, ni émission de session.
- **Lazaret :** issuer Kubernetes et claims Pod/SA du vérificateur offline
  `runtime_identity.rs` : B1/§13.1-B. Ce n'est ni TokenReview ni client cluster,
  et cela ne remplace pas le recoupement du registre authoritative.
- **P4 :** clés publiques épinglées, vérification locale, challenges/guards et
  journal d'admission : §13.5-A/C/D. Le littéral policy ID
  `apparatus-p4-conformance/v1` est ratifié, pas un moteur de conformance.

## Interdits inchangés et preuve exigée

Pas de client/token Kubernetes, host/plugin engine ou I/O utilisateur dans
Manifesto ; pas de client cluster dans Lazaret ; pas de signer privé, Cosign/ORAS,
mint de VALID ni exécution de conformance dans Manifesto. Rôles librement fournis,
bypass legacy et autorisation par rôle seul restent interdits.

Les six targets historiques doivent passer avec des témoins synthétiques
positifs et négatifs : fichier non autorisé, identifiant presque identique,
appel interdit ajouté sur une ligne autorisée, client cluster ajouté au
vérificateur d'enrollment, et véritable méthode d'exécution dans le contrôle.
Les inscriptions `/components`, contrôles anti-retarget et tests du trait sont
conservés. Aucune nouvelle décision produit n'est nécessaire pour reconnaître
ces seules interfaces déjà ratifiées ; lever un invariant exigerait l'humain.

## Limites

Aucune violation structurelle n'est établie parmi les occurrences CI examinées.
Ce constat n'est ni un audit de sécurité global ni un PASS des treize targets.
L'analyse architecte n'a exécuté aucun test/runtime et n'a modifié aucun fichier.
La preuve finale appartient au diff et aux validations du lead ; cette note
reste une proposition, pas un Accept d'allowlist automatique.
