---
title: "Apparatus — capacités, identité et isolation"
category: concepts
tags: [components, permissions, security, openfga, visibility/internal]
status: proposed
feature_status: future
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - openfga/model.fga
  - Manifesto/application/src/usecase/world_read.rs
  - Manifesto/infra/src/transaction.rs
  - Manifesto/config/default.toml
  - https://kubernetes.io/docs/concepts/security/pod-security-standards/
  - https://kubernetes.io/docs/concepts/services-networking/network-policies/
summary: "Autorisation future par intersection de droits, gateway de capacités, identité de workload dédiée, stockage par binding et limites des sandboxes."
provenance:
  extracted: 0.18
  inferred: 0.82
  ambiguous: 0.0
created: 2026-09-09T17:50:00Z
updated: 2026-09-09T17:50:00Z
---

# Apparatus — capacités, identité et isolation

Cette conception appartient à [[projects/manifesto/concepts/apparatus-platform]]. Le dépôt possède l’authentification utilisateur et les ACL projet/composant ; il ne possède pas encore le capability gateway, le stockage dédié ni l’identité de workload ci-dessous.

## Droits effectifs proposés

Pour un appel interactif, autoriser seulement l’intersection suivante : utilisateur actuellement autorisé sur le projet **et** l’instance, capacité déclarée par la release, capacité consentie au binding, politique de l’organisation/plateforme, état courant du binding et restriction propre à l’opération. Aucun de ces contrôles ne peut être remplacé par un bouton masqué dans l’UI. ^[inferred]

L’identité utilisateur est extraite de la session du host et vérifiée côté serveur. Le gateway résout `project_id`, organisation, release et permissions depuis le binding ; il ignore toute tentative du plugin de substituer ces identifiants. Une requête comporte une opération nommée et des paramètres validés, pas une URL interne arbitraire. ^[inferred]

| Capacité candidate | Autorité / contrainte proposée |
|---|---|
| `project.read` | DTO minimal du projet après porte projet + porte composant |
| `project.metadata.write` | Liste explicite de champs ; jamais propriétaire, visibilité, membres ou ACL par raccourci |
| `organization.members.read` | Autorisation organisationnelle Hive et projection minimale des membres |
| `storage.kv.read/write` | Namespace imposé par le binding, quota et taille de valeur |
| `network.github-api` | Destination, méthodes et chemins fixés par un connecteur approuvé |
| `notifications.emit` | Destinataires autorisés, modèle de message et quota ; pas de messagerie arbitraire |

Cette taxonomie est un projet de contrat. Toute capacité inconnue est refusée. Les relations OpenFGA existantes ne couvrent pas à elles seules cette granularité : conserver OpenFGA pour « qui peut agir sur le projet/composant », et stocker le consentement de capacités dans le control plane. ^[inferred]

## Ne pas hériter involontairement de la visibilité publique

Aujourd’hui, la lecture passe par `enforce_world_read_or_principal`, puis `caller_can_read_component`. Un projet public et actif ouvre la lecture de ses composants. Cela ne doit pas rendre automatiquement publics les secrets, le stockage ni toutes les actions d’un futur Apparatus. Voir [[projects/manifesto/concepts/org-owned-visibility-and-participation-limits]] et [[projects/manifesto/concepts/component-instance-permissions]].

Proposition V1 : aucun invoke anonyme par défaut. Une éventuelle contribution publique expose uniquement des opérations de lecture explicitement marquées publiques par la release et activées par l’administrateur du projet. Filtrer également les nouveaux DTO : ne pas ajouter configuration ou références de secrets à la réponse legacy accessible publiquement. ^[inferred]

Les mutations conservent le contrôle immédiat des membres en DB décrit dans [[projects/manifesto/concepts/immediate-membership-acl]]. Une ancienne permission OpenFGA ou un cache de session ne peut pas maintenir une autorisation après suspension du membre. ^[inferred]

## Identités et secrets de la plateforme

Le contrat interne utilisateur du dépôt repose actuellement sur JWT HS256, `iss=iamrusty`, `aud=aiforall`. Un secret HMAC partagé permettrait de fabriquer des identités : il ne doit jamais entrer dans un conteneur tiers, une étape de build ou un bundle UI.

Le protocole proposé sépare : ^[inferred]

- **Host → API plateforme** : session utilisateur habituelle, conservée dans la frontière de confiance.
- **Gateway → Apparatus** : contexte réduit de l’appel, signé par une autorité Apparatus dédiée si nécessaire ; pas le bearer IAM original.
- **Apparatus → Gateway** : identité de workload propre à l’instance, échange contre un jeton court exclusivement destiné au gateway. Claims minimaux : instance, binding, release, génération, révision de droits, audience et expiration.
- **Contrôleur → runtime** : credential d’administration du runtime réservé au contrôleur.
- **Observations runtime** : canal authentifié contrôleur/agent ; le plugin peut rapporter sa santé, mais ne décide ni de ses droits ni de sa release admise.

Le mécanisme concret recommandé pour la V1 est un certificat client mTLS délivré par une autorité Apparatus, puis un jeton de session du gateway signé avec une clé dédiée. La distribution initiale, la rotation et l’expiration de ce certificat doivent être implémentées et testées ; l’application ne reçoit aucun token Kubernetes. Les clés privées restent un secret de workload limité, distinct de tout secret utilisateur ou cloud. ^[inferred]

Le gateway vérifie `grant_revision`, génération et suspension au moment de l’appel ; un JWT valide cryptographiquement ne suffit pas à garantir que son droit est toujours accordé. Les callbacks appartiennent au binding résolu par l’identité, pas au projet déclaré dans leur JSON. ^[inferred]

## Tâches de fond

Un Apparatus peut avoir besoin de synchroniser sans utilisateur connecté. Ne pas simuler le propriétaire par un JWT longue durée. Un grant de service explicite, accordé lors de l’installation par un administrateur autorisé, limite opérations, données et durée. Il appartient au projet/organisation, est audité et peut être révoqué indépendamment de la session de l’installateur. Les changements de propriétaire/organisation déclenchent une réévaluation ; une politique organisationnelle ne doit pas survivre implicitement à un transfert. ^[inferred]

La V1 n’autorise pas de publication directe sur SQS/Kafka ni d’accès direct à OpenFGA, PostgreSQL ou Redis. Un webhook externe éventuel arrive sur un ingress plateforme dédié, vérifie la signature du fournisseur, déduplique puis déclenche une opération bornée. L’endpoint du conteneur n’est pas rendu public. ^[inferred]

## Isolation des workloads et builds

Les recommandations suivantes sont une politique future à vérifier par tests adversariaux. Un conteneur Linux ne constitue pas à lui seul une frontière suffisante contre du code communautaire hostile. ^[inferred]

- Un workload par binding ; CPU, mémoire, stockage temporaire, nombre de processus et durée des opérations bornés.
- Utilisateur non-root, filesystem racine en lecture seule, volume temporaire plafonné, `allowPrivilegeEscalation=false`, capacités Linux supprimées, seccomp, aucun hostPath/hostNetwork/socket Docker.
- Pas d’automount de token de service account, pas de secrets plateforme ni de credentials cloud ; seuls les credentials restreints du gateway sont montés.
- Nœuds ou runtime sandbox séparés de la plateforme pour les workloads tiers et les builds, avec une isolation de type sandbox renforcée ou VM à qualifier. Le choix du moteur est un arbitrage d’infrastructure, pas une option libre du manifeste.
- Politique réseau deny ingress/egress par défaut ; ingress depuis le gateway/contrôleur, egress vers le gateway et la résolution strictement nécessaire.
- Profil opérateur validé avant admission. Si l’isolation requise n’est pas disponible, refuser l’installation plutôt que déployer avec des protections réduites.

Le standard Kubernetes **Restricted** fournit un socle de contraintes de conteneur, notamment non-root, seccomp et suppression des capacités. Il ne remplace ni les quotas, ni une politique réseau, ni une frontière entre tenants. [Source officielle](https://kubernetes.io/docs/concepts/security/pod-security-standards/).

## Réseau et données sortantes

Une NetworkPolicy Kubernetes dépend du plugin réseau et travaille notamment sur pods, namespaces et blocs IP. Elle n’implémente pas à elle seule un proxy HTTP vérifiant un domaine, un chemin ou une méthode. [Source officielle](https://kubernetes.io/docs/concepts/services-networking/network-policies/).

Proposition : les capacités réseau passent par un proxy/gateway de confiance. Il impose HTTPS, destination et port connus, méthode/chemin autorisés, limites de payload/réponse/durée, et secrets upstream sélectionnés depuis le binding. Il refuse URL arbitraire, CONNECT et redirections non revalidées ; résolution et connexion doivent bloquer réseaux privés, loopback, link-local, metadata cloud et équivalents IPv6, y compris après résolution DNS ou redirect. ^[inferred]

Un domaine autorisé peut lui-même recevoir des données sensibles. La permission réseau doit donc annoncer l’usage et la destination ; réduire les données accessibles au plugin reste nécessaire. Ni CSP, ni allowlist, ni statut `VALID` ne prouvent l’absence d’exfiltration ou de comportement malveillant. ^[inferred]

## Stockage et configuration

Choix V1 proposé : backend stateless et données durables via une API KV plateforme, plus blobs si un cas concret l’exige. Les bases des services actuels restent inaccessibles. Les tables de cette nouvelle API appartiennent à la plateforme ; l’identité impose un préfixe logique `binding_id`, sans paramètre permettant d’accéder à un autre namespace. ^[inferred]

Le contrat KV précise taille maximale, quota total, versions/CAS, pagination et erreurs. Le quota est appliqué atomiquement côté service pour éviter deux écritures concurrentes le dépassant. Les clés, logs, objets, exports et sauvegardes portent aussi le binding ; un préfixe dans une chaîne fournie par le plugin ne constitue pas un contrôle d’accès. ^[inferred]

Les migrations d’Apparatus portent sur leurs données KV via cette API, jamais sur le schéma SQL de Manifesto ni sur un accès administrateur à PostgreSQL. L’extension déclare `data_schema_version` et compatibilité minimale/maximale ; voir [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]. ^[inferred]

Les secrets upstream sont chiffrés par un service de secrets plateforme à sélectionner. La configuration conserve une référence opaque ; le gateway injecte le secret uniquement dans l’appel approuvé au fournisseur. Ne pas renvoyer les secrets au plugin « pour simplifier le SDK ». Les exports, sauvegardes, quotas et rétention constituent des opérations administratives auditées. ^[inferred]

## Consentement, révocation et audit

Afficher capacité, données, destination et mode d’exécution à l’installation. Enregistrer auteur, date, release et empreinte du jeu de permissions. Tout ajout de capacité ou élargissement de destination attend un nouveau consentement ; un badge `VERIFIED` ne l’accorde pas. ^[inferred]

Une révocation incrémente la révision des grants, bloque les nouvelles opérations, ferme les bridges et déclenche la suspension runtime. Auditer binding, release, acteur ou grant de service, opération, résultat et corrélation ; jamais tokens, secrets ou corps sensibles. Les logs venant d’un Apparatus restent non fiables et sont bornés/sanitisés. ^[inferred]

La suite d’acceptation dans [[projects/manifesto/references/apparatus-implementation-plan]] doit prouver ces limites avant toute admission communautaire.

