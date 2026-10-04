# ADR-0605 : Le gold path local Kind = HTTP 200 via le monolithe J3 seul Lazaret ; DNS = Service ClusterIP du même nom que le Pod ; attach digest + declared_capabilities = writer managed existant

- Statut : Proposed
- Réalité : Partial
- Date : 2026-09-25
- Décideurs : (à remplir à l’acceptation)
- Jalon concerné : Cloud-portable (gold path local Kind ; hors Apparatus P0–P6)
- SuperSède : aucune
- SuperSédée par : —
- Related : [0601](0601-cluster-trust-namespaces-standalones.md), [0604](0604-j3-overlay-demo-monolith-kind-invoke.md), [0603](0603-tranche-locale-deploy-kind-apparatus-lazaret.md), [0006](0006-apparatus-p2-reconciliation-in-process.md), [0007](0007-apparatus-p3-capability-boundary-after-accept.md), [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md), [0010](0010-gate-preprod-workload-certificate-ca.md), [0404](0404-runtime-microservices-et-monolithe.md)

`Accepted` ratifierait le **gold path local** ci-dessous ; le Statut reste Proposed. `Réalité : Partial` au 2026-10-04 : attach writer, locator DNS, Service operator, Job Adm-A et helper présents ; l’ancien Implemented et HTTP200 J3 restent des preuves historiques, pas une validation du root actuel modifié. D-TRANSIT-TCB (OpenBao DEV commun) reste visible. Cette ADR ne SuperSède ni [0601](0601-cluster-trust-namespaces-standalones.md), ni [0604](0604-j3-overlay-demo-monolith-kind-invoke.md), ni [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md), ni [0010](0010-gate-preprod-workload-certificate-ca.md). Le nouveau lab [0606](0606-local-full-kind-isole.md) ne constitue pas automatiquement une nouvelle preuve gold/J3.

## Contexte

Le guide [platform-local-gold-case-implementation-guide.md](../platform-local-gold-case-implementation-guide.md) laissait ouvertes : gold path J1 host vs J3 in-cluster ; hop / locator (L6) ; qui peuple `digest` / `declared_capabilities` à l’attache. [0604](0604-j3-overlay-demo-monolith-kind-invoke.md) prouve plugins → nest monolithe in-cluster **sans** ériger le monolithe en unité prod. [0601](0601-cluster-trust-namespaces-standalones.md) reste le canon cluster 4+1. Curl hôte `127.0.0.1:8080` et `StaticPluginLocator` + URL écrite à la main sont des **faux amis** / dette, pas la cible. Certificat / CA et scale / pod partagé sont déjà gated par [0010](0010-gate-preprod-workload-certificate-ca.md) et [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md) — **citer, ne pas réécrire**.

## Décision

Les trois points ci-dessous forment **un seul** choix de gold path local Kind (option B du guide).

1. **Gold path = HTTP 200 sur Kind (option B / J3).** Le monolithe chargé dans `aiforall-local` est le **seul** Lazaret du parcours. Enroll, session mTLS et invoke passent par **ce** processus. Curl hôte `127.0.0.1:8080` n’est **pas** la preuve. [0601](0601-cluster-trust-namespaces-standalones.md) (canon 4+1) et [0604](0604-j3-overlay-demo-monolith-kind-invoke.md) (overlay démo = écart de séquence) **restent**. Le gold path s’appuie sur l’overlay **sans** le déclarer unité prod.

2. **Résolution d’adresse.** L’operator crée un Service ClusterIP **du même nom que le Pod** plugin. Lazaret **ne parle pas** à l’API kube. Hostname = `plugin-{32 premiers hex du digest descripteur}` (code `pod_name_for`). URL de base : `http://plugin-{32 premiers hex du digest descripteur}.apparatus-plugins.svc:8080`. L’opération (`kv.get`) est dans le **corps** HTTP invoke (pas gRPC, pas d’id dans le chemin). Le binding UUID = identité de grant, **pas** le hostname. Digest comme clé DNS = **valable pour ce gold path Kind**. **Interdit** de le figer comme modèle prod : [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md) — la clé pourra devenir binding/instance ; Lazaret ne change que la **formule**, pas un client kube. `StaticPluginLocator` + URL écrite à la main = **dette**, pas la cible.

3. **Capabilities et digest à l’attache.** Le writer managed existant (`POST …/components` → `insert_managed_binding`) remplit `digest` (pin catalogue / descripteur) et `declared_capabilities` (manifeste du type). Consents = PUT admin T5 ([0007](0007-apparatus-p3-capability-boundary-after-accept.md)). Pas de nouvelle route `/components` ([0006](0006-apparatus-p2-reconciliation-in-process.md) E). Pas de seed SQL. Pas d’URL domaine inventée ([0007](0007-apparatus-p3-capability-boundary-after-accept.md) point 5) dans cette décision. Ni le plugin ni Adm-A n’écrivent ces colonnes.

4. **Déjà enregistré — citer, ne pas réécrire.** Certificat / CA = [0010](0010-gate-preprod-workload-certificate-ca.md). Scale / pod partagé = [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md).

## Conséquences

- Guide gold case : J1 vs J3, L6 hop/locator, et « qui peuple declared/digest » sont **fermés** par cette ADR ; CSR reste → 0010 ; HPA/mécanisme → 0009.
- Coding : réaliser le parcours Kind J3 jusqu’à HTTP 200 métier ; Service ClusterIP nom=Pod ; writer managed remplit digest + declared ; **pas** client kube dans Lazaret ; **pas** seed SQL ; **pas** nouvelle route `/components`.
- Accept de cette ADR = obligation de traiter le gold path local selon ces trois points ; **pas** un SuperSéde de 0601/0604 ni un modèle prod pour la clé DNS.
- **Écart 0604 :** [0604](0604-j3-overlay-demo-monolith-kind-invoke.md) « Calico hors J3 » reste vrai pour le livrable J3 (overlay démo inchangé). L’enforcement CNI = Calico v3.29.7 sur `aiforall-local` seulement (gold path local) ; pas de SuperSéde 0604.
- **Schedule fail-closed :** l’operator re-vérifie Cosign (`verify_cosign_signature`, même pin / Transit / zot que Adm-A) avant `pods.create`. Un VALID JSON sans signature valable ne schedule pas. Controller = verify only. Delta prod **D-TRANSIT-TCB** (déjà [0008](0008-apparatus-p4-k8s-isolation-outside-manifesto.md)) : remplacer OpenBao `-dev`, séparer Transit du KV plugin, seul SA `admit-sign` appelle Transit pour signer — local `-dev` reste OK.

## Alternatives rejetées

| Option | Pourquoi pas (maintenant) |
|---|---|
| Gold path = J1 host (`127.0.0.1:8080`) comme preuve | Ne prouve pas le Lazaret qui enroll/invoke in-cluster ; faux ami 0604 |
| Client kube dans Lazaret pour résoudre le plugin | Contredit le handbook locator ; Lazaret ne parle pas à l’API kube |
| Figé digest-as-DNS comme modèle prod | Contredit la gate 0009 (clé pourra devenir binding/instance) |
| StaticPluginLocator + URL manuelle comme cible | Dette observée, pas le gold path |
| Nouvelle route `/components` ou seed SQL pour digest/declared | Contredit 0006 E et 0007 ; seed ≠ solution finale |
| Plugin ou Adm-A écrivent digest / declared_capabilities | SoT = writer managed à l’attache ; Adm-A = VALID seulement |
| Saucissonner en trois ADR | Un seul choix de gold path local |

## Non décidé ici

- Modalités CSR / produit CA (renvoie [0010](0010-gate-preprod-workload-certificate-ca.md) / [0007](0007-apparatus-p3-capability-boundary-after-accept.md))
- Mécanisme HPA / KEDA / pod-par-binding / scale-to-zero (renvoie [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md))
- Clé DNS prod (binding vs instance vs digest) — hors gold path Kind
- J4 bascule 4+1 ; GKE / 0602 ; Factory / host UI (P5/P6)
- Contenu exact YAML / impl locator (formule URL seulement)

## Réalité courante et preuve historique — 2026-10-04

- La preuve `just prove-gold` HTTP200 décrite en références est historique. Source présente : writer managed attach, locator DNS/operator, Job `ops/deploy/p4/admit-job.yaml` et overlay `ops/deploy/apps/overlays/kind-demo-monolith/`. Aucun nouveau build/IT/E2E de ce parcours au root HEAD `5348a63` modifié/non committé ne permet de maintenir Implemented comme réalité courante.
- La cible métier reste attach digest/declared via writer managed, URL Service du Pod, Adm-A signé et invoke in-cluster. Le rendu D6/local-full 112 ressources ne prouve ni l’attache, ni la validation admission, ni HTTP200. Le choix monolithe J3 n’est pas réinterprété silencieusement comme preuve standalone.
- Le lab local-full dédié ([0606](0606-local-full-kind-isole.md)) vise dépendances in-Kind et PVC local-path ; DEV/SQS volatils, catalog/SMTP simulés, OAuth externe désactivé et stockage node-local ne sont ni HA ni reprise prouvée. Preuve crypto/autorisation/attache/réseau et backup/restore cible distincte restent exigés selon leurs constats, sans retrait automatique du scope local.
- Root compilation/units, IT testcontainers puis E2E après intégration : contexte full autorisé et inventaire/bail vérifiés. Legacy étranger préservé ; non-reproductibilité gold/J3 dans ce périmètre ⇒ INCONCLUSIVE/arbitrage humain, jamais delete/recreate ni transfert de résultat historique. Aucun cloud/GKE/Flux/HSM fictivement livré.

## Références

- Guide : `docs/platform-local-gold-case-implementation-guide.md` ; plan : `docs/platform-local-monolith-kind-implementation-plan.md`
- Overlay démo : [0604](0604-j3-overlay-demo-monolith-kind-invoke.md) ; canon cluster : [0601](0601-cluster-trust-namespaces-standalones.md)
- Gates pré-prod (citer) : [0009](0009-gate-preprod-scale-on-demand-isolation-instance.md), [0010](0010-gate-preprod-workload-certificate-ca.md)
- Attach / consents : [0006](0006-apparatus-p2-reconciliation-in-process.md), [0007](0007-apparatus-p3-capability-boundary-after-accept.md)
- Preuve historique de la tranche gold/J3 (pas de nouvelle exécution) : `just prove-gold` ; Job `apparatus-admit-gold-*` SA `admit-sign` ; Pod+Service `plugin-{32hex}` ; invoke in-cluster HTTPS `/lazaret/invoke` → HTTP 200 + `hostname=plugin-{32hex}`
