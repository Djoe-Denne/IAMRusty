---
title: "Apparatus — UI, SDK et protocole"
category: references
tags: [components, frontend, security, api, visibility/internal]
status: proposed
feature_status: future
sources:
  - "C:/Users/djden/.codex/attachments/486d0052-5759-4277-bcc1-9f209ce353d4/pasted-text.txt"
  - Manifesto/http/src/lib.rs
  - apparatus-events/src/component.rs
  - https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/iframe
  - https://developer.mozilla.org/en-US/docs/Web/API/Window/postMessage
summary: "Host UI encore absent, contrat statique, sandbox cross-origin, bridge de capacités, protocole backend versionné et conformance future."
provenance:
  extracted: 0.3
  inferred: 0.7
  ambiguous: 0.0
created: 2026-09-09T17:50:00Z
updated: 2026-09-09T17:50:00Z
---

# Apparatus — UI, SDK et protocole

Manifesto est actuellement une API Rust. L’audit n’a trouvé ni frontend produit Manifesto, ni host de slots, ni bridge, ni design system ; le React de `IAMRusty/qa/test-ui` est une fixture QA. Il faut donc créer le host avant d’intégrer les UI de [[projects/manifesto/concepts/apparatus-platform]].

## Sortie frontend et slots

Le document demande un résultat statique, indépendant du framework, sans serveur Node permanent par Apparatus. Proposition V1 : un builder Vite produit `index.html` et des assets locaux ; les applications nécessitant SSR ou serveur privé ne sont pas admises. Les futurs builders devront produire le même contrat. ^[inferred]

Le bundle est servi depuis un digest immuable. Pas d’URL de script distante, de chemin sortant de l’artifact, de source map publique contenant des secrets, ni d’accès direct au backend du plugin. Une route client comme `/repositories` est résolue dans la base de l’artifact ; le serveur fait un fallback SPA contrôlé pour ces routes, jamais pour un asset inexistant ou un chemin arbitraire. ^[inferred]

| Mode | Proposition V1 |
|---|---|
| `schema` | Formulaires/settings et widgets limités, rendus par le host avec schéma versionné ; pas de HTML/script/eval ou expression arbitraire |
| `sandbox` | Application statique dans une iframe ; interactions plateforme exclusivement via le bridge |
| `component` | Reporté : un composant ou Web Component exécuté dans le host partage son contexte JavaScript et n’offre pas la même frontière |

Les slots sont une liste blanche : `project.tab`, `project.settings`, `project.overview.widget`. Le host décide ordre, taille et droits d’affichage. Identifiants de contribution uniques par Apparatus ; titres rendus comme texte, icônes/URLs validées. Aucun remplacement de la navigation principale ou des contrôles d’administration. ^[inferred]

## Origine et sandbox proposées

Servir le host, par exemple, sur `app.aiforall.example`, et les UI sur des origines opaquement nommées `u-<session>.apparatus.example`, donc sur un autre domaine enregistrable. Affecter une origine distincte à chaque session d’affichage/binding évite qu’un stockage navigateur partagé par tout un publisher traverse les projets. Tous les assets sont hébergés par la plateforme ; aucun cookie d’authentification du host ne doit être valide sur le domaine Apparatus. ^[inferred]

Pour supporter les modules JavaScript statiques et une validation d’origine exacte, proposer `sandbox="allow-scripts allow-same-origin"` **uniquement** sur cette origine réellement distincte. Ne pas autoriser forms, popups, top-navigation, downloads, workers ou fonctions sensibles par défaut. Ajouter titre accessible et `referrerpolicy="no-referrer"`. ^[inferred]

Sans `allow-same-origin`, l’iframe acquiert une origine spéciale ; le protocole doit alors gérer une origine opaque. Avec `allow-scripts` et `allow-same-origin` sur une page de même origine que le parent, la sandbox est contournable. D’où le choix explicite d’un domaine distinct, imposé et testé par le host. [Référence iframe](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/iframe).

Headers du serveur d’assets à définir par le builder et le host : CSP deny-by-default, scripts/styles/assets limités à l’origine d’artifact, `connect-src 'none'`, `object-src 'none'`, `base-uri 'none'`, `form-action 'none'`, `frame-src 'none'`, `worker-src 'none'`, `frame-ancestors` sur l’origine host exacte. Les éventuels styles inline nécessitent un profil CSP délibéré et testé ; ne pas affaiblir toute la politique pour faire passer un bundle. Permissions Policy refuse caméra, micro, géolocalisation et fonctions similaires. ^[inferred]

L’iframe peut tenter une navigation de son propre document ; CSP et sandbox ne constituent pas une garantie générale d’absence de fuite des données qu’on lui a volontairement remises. Le protocole ferme le canal après navigation et limite les données délivrées. Les tests multi-navigateurs doivent mesurer cette limite ; les exigences de confinement plus fortes imposeront le mode schema ou une autre technologie. ^[inferred]

## Bridge : initialisation et messages

Le SDK ne contient pas d’autorité : il facilite un protocole que le host et le serveur vérifient. L’API `postMessage` impose de choisir une origine cible exacte et de vérifier l’expéditeur ; une fenêtre peut avoir navigué depuis un précédent message. [Référence postMessage](https://developer.mozilla.org/en-US/docs/Web/API/Window/postMessage).

Proposition de handshake : ^[inferred]

1. Le host vérifie la session, les droits et la release active, crée l’iframe avec URL vérifiée et une session de bridge côté serveur.
2. L’iframe annonce `hello` avec versions supportées ; le host vérifie `event.source === iframe.contentWindow` et `event.origin === origine_attendue`.
3. Le host établit un `MessageChannel` vers cette origine exacte, avec nonce de session. L’iframe vérifie symétriquement origine/source du parent. Le nonce sert à la corrélation, jamais de JWT.
4. Le port est associé dans le host à un binding/release/session fixé. Messages validés par schéma et allowlist ; le serveur revalide l’autorisation pour chaque opération.
5. Navigation, reload, changement de projet, logout, révocation ou upgrade ferment le port, annulent les demandes et exigent un nouveau handshake.

Côté serveur, chaque session de bridge est liée à `(binding_id, release_descriptor_digest, user_session_id, grant_revision)`. Rejeter une demande si cette association est périmée, même si le port n’est pas encore fermé. Revalider avant une mutation et avant la livraison d’une réponse sensible ; un effet déjà exécuté ne peut pas être annulé rétroactivement. ^[inferred]

Exemple de message cible, non implémenté : ^[inferred]

```json
{
  "protocol": "manifesto-apparatus-ui/1",
  "session": "opaque-session-id",
  "request_id": "opaque-request-id",
  "method": "project.get",
  "params": {}
}
```

Le host ne fait pas confiance à un `project_id`, un rôle ou une permission transmis dans `params`. Il ne possède pas de méthode `fetchInternal(url)`. Le dispatch associe chaque méthode à sa capacité, au DTO autorisé et au service cible. Le backend renvoie soit `result`, soit `error.code` parmi un vocabulaire stable : `permission_denied`, `binding_not_ready`, `quota_exceeded`, `timeout`, `invalid_request`, `unavailable`. ^[inferred]

Valeurs de départ proposées, configurables et à mesurer : message JSON équivalent ≤256 KiB, 32 requêtes en vol, timeout interactif 10 s. Les opérations longues renvoient un identifiant de suivi ; transferts volumineux passent par une API dédiée à autorisation bornée. Déduplication par session/request_id et clé d’opération serveur pour les effets persistants ; annuler l’attente UI ne garantit pas l’annulation d’un effet déjà accepté. ^[inferred]

Thème, locale et densité sont des métadonnées validées, sans secret. Le package UI officiel et les design tokens sont facultatifs pour les sandboxes ; schema reste rendu par le host. Événements de thème et notifications sont limités, avec quotas et rendu sans HTML arbitraire. ^[inferred]

## Protocole backend cible

Les endpoints suivants appartiennent au **workload privé**. Ils ne sont ni de nouvelles routes existantes de Manifesto, ni des URLs remises au navigateur. Ils devront être normalisés dans un contrat versionné. ^[inferred]

| Opération | Contrat candidat |
|---|---|
| Découverte | `GET /.well-known/apparatus` : ID, version, protocole, fonctions, empreinte du manifeste |
| Liveness / readiness | `GET /health`, `GET /ready` : processus et capacité de service, sans données sensibles |
| Bind | `POST /v1/bindings/{id}/bind` : opération, génération, configuration validée, références autorisées |
| Configure | `PUT /v1/bindings/{id}/configuration` : génération/config revision ; idempotent |
| Invoke | `POST /v1/bindings/{id}/invoke` : opération nommée, contexte minimal et paramètres |
| Unbind | `POST /v1/bindings/{id}/unbind` : nettoyage logique idempotent |
| Shutdown | Signal standard avec drainage borné, puis arrêt forcé par le runtime |

Le contrôleur authentifie le workload et compare ID/version/empreinte au descripteur signé attendu. Un manifeste auto-déclaré ne permet pas d’augmenter des permissions. Les endpoints sensibles vérifient l’identité du contrôleur/gateway, l’instance, le binding et la génération. La configuration et les hooks utilisent `operation_id` pour rejouer proprement. ^[inferred]

Le SDK peut exposer des traits/hook Rust correspondant à ces opérations ; la macro `#[manifesto::apparatus]` du document est une idée d’ergonomie, pas une API existante. L’identité de workload et les capabilities doivent rester utilisables sans dépendre de cette macro. ^[inferred]

Ne pas étendre directement `apparatus_events::ComponentStatusChangedEvent` en protocole général : il adresse aujourd’hui `project_id + component_type` sans binding ni génération. Le nouveau lifecycle versionné passe par le contrôleur et la réconciliation décrits dans [[projects/manifesto/concepts/apparatus-bindings-and-lifecycle]]. ^[inferred]

## Conformance

La suite plateforme doit tester le protocole sans données réelles : versions incompatibles, réponse de handshake mensongère, double bind/unbind, interruption réseau, shutdown, timeouts, chargement SPA, slots inconnus, message trop grand, mauvaise origine/source, rechargement, changement de tenant, refus de capacité et révocation en session. Un artifact qui s’affiche mais contourne un refus n’est pas conforme. ^[inferred]

Voir [[projects/manifesto/references/apparatus-implementation-plan]] pour les jalons et [[projects/manifesto/concepts/apparatus-capabilities-and-isolation]] pour l’autorité réelle du gateway.

## Liens avec l’existant

- [[projects/manifesto/references/manifesto-api-and-permission-flows]] — routes et permissions actuelles.
- [[projects/manifesto/references/manifesto-event-model]] — consumer et événements actuels.
- [[projects/manifesto/references/apparatus-factory-and-distribution]] — production du bundle et politique de build.
