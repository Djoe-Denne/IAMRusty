---
title: TLS, HMAC et signatures — protections complémentaires
category: concepts
tags: [security, oauth, authentication, architecture, visibility/internal]
sources:
  - docs/guides/iam-oauthconnect-securite-clefs-jwks-tls.md
  - docs/guides/iam-oauthconnect-securite-sources.md
  - services/OAuthConnect/setup/src/transport.rs
  - services/IAMRusty/infra/src/auth/http_connector.rs
  - docs/missions/20261108-ops-oauthconnect.md
  - ops/deploy/render-local-full.py
summary: >-
  TLS protège un hop et son serveur ; HMAC authentifie une requête interservices ; OIDC signe une identité contextualisée. Aucun ne remplace les deux autres.
provenance:
  extracted: 0.97
  inferred: 0.03
  ambiguous: 0.0
created: 2026-11-08
updated: 2026-11-08
---

# TLS, HMAC et signatures

## Trois questions différentes

**TLS :** à quel serveur suis-je connecté et qui peut observer/modifier les octets sur ce hop ? **HMAC S2S :** la requête exacte vient-elle d'un détenteur du secret de cette instance ? **Signature OIDC :** cette autorité a-t-elle signé cette identité pour le client et le contexte attendus ? Une couche ne répond pas à toutes les questions.

TLS apporte confidentialité et intégrité du canal. Un JWT signé est généralement lisible ; sa signature n'en chiffre pas les claims. HMAC partage un secret entre signataire et vérificateur : chacun peut calculer un MAC valide, contrairement à la séparation privé/public d'une signature asymétrique.

## Les cinq familles à ne jamais échanger

1. **Privé fournisseur OIDC** : signe l'ID token ; le JWKS public va à Connect.
2. **Signer JWT IAM** : signe les access tokens plateforme ; le JWKS IAM va à leurs consommateurs.
3. **Secret HMAC d'instance** : authentifie IAM↔Connect ; ni clé JWT ni matériau public.
4. **Clés/certificats TLS** : prouvent l'identité de l'extrémité du canal ; ne signent pas automatiquement les JWT.
5. **Client secret OAuth** : authentifie le client auprès du token endpoint selon son profil ; ne permet pas de fabriquer un ID token signé par le fournisseur.

Les credentials Connect KVv2/direct sont résolus au boot et conservés dans le catalogue immuable ; un refresh JWKS ne relit pas ces secrets ni ne les tourne. Transit de signature privée IAM est une autre fonction qu'un coffre KV de client secrets.

## Handshake TLS1.3, version pédagogique

Le client propose des paramètres et une contribution éphémère ; le serveur répond avec les siens. Ils dérivent des secrets de session depuis cet échange, distincts de la clé longue durée du certificat. Le serveur présente sa chaîne de certificats, prouve la possession du privé et authentifie le transcript du handshake ; les messages Finished confirment ce transcript. Le trafic applicatif est ensuite protégé par un chiffrement symétrique authentifié (AEAD).

Le client vérifie notamment la chaîne vers une racine admise, les usages/dates et le **nom du serveur dans SAN**. SNI aide le serveur à choisir sa configuration ; ce n'est pas une preuve de validité. Une CA de confiance pour un autre nom ne suffit pas. En mTLS le serveur exige en plus une identité certificat du client ; HTTPS classique ne prouve pas à lui seul cette authentification mutuelle.

Cette explication du protocole ne prouve pas que chaque environnement emploie TLS1.3, mTLS, ACME ou cert-manager.

## Raisonner hop par hop

- Navigateur→IAM ou son terminator : protéger cookies/code et authentifier le site ; une terminaison proxy crée une frontière, pas du TLS magique jusqu'au binaire.
- IAM→Connect : vérifier l'extrémité Connect, protéger corps et HMAC ; Connect vérifie le MAC de la requête exacte.
- Connect→Discovery/JWKS/token/UserInfo/API vendor : vérifier les serveurs admis, protéger code, credentials et tokens ; puis vérifier séparément l'identité OIDC.

Si Envoy termine TLS, le hop suivant doit être décrit explicitement comme TLS/mTLS ou clair. Les configurations historiques HTTP interne et la cible HTTPS ne constituent pas une preuve de chiffrement de bout en bout live.

Application au profil local `--oauth-connect` livré le8novembre : l'entrée
`https://127.0.0.1:18080` utilise le SAN IP de la feuille Envoy existante sans
exiger de certificat client navigateur. Le changement reste downstream ; les
transport_socket mTLS upstream sont conservés. Callbacks HTTPS, origine/cookie
IAM et issuer des consommateurs sont alignés, sans exception dans le validateur
Connect et sans passer GitHub en mode insecure. Les profils legacy gardent leur
entrée HTTP. Ce sont des configurations/revues offline, pas des handshakes live.
Voir [[projects/oauthconnect/concepts/local-ops-and-secret-distribution]].

## Deux builders, deux exceptions différentes

IAM→Connect Verified exige HTTPS et garde la validation du certificat/nom, même avec des racines additionnelles. Les exceptions d'URL HTTP de ce constructeur ne désactivent pas les vérifications d'un endpoint HTTPS.

Le builder vendor Connect est Verified par défaut. Ses modes explicites `LocalInsecure`/`IsolatedTest` activent en revanche `danger_accept_invalid_certs/hostnames(true)`. Ce n'est **pas** une simple CA privée supplémentaire ; ne pas extrapoler ce comportement au builder IAM. L'exception n'est pas inférée du nom localhost ou du type de build.

Une CA additionnelle en mode vérifié étend les racines admises sans retirer le contrôle du hostname. Ne pas désactiver CA/SAN pour faire passer une fixture. Les tests synthétiques de mauvaise CA, mauvais SAN et redirect refusé restent des preuves locales, pas des captures de certificats en production.

Pour le profil ops, la confiance navigateur envers `mesh/ca.crt` et le callback
HTTPS enregistré chez GitHub restent à vérifier explicitement. L'extension PKI
OpenBao/Connect ne constitue ni une installation de confiance système ni une
rotation de clés IAM. Le hop HTTP `simulated` reste une exception isolée de test,
pas une relaxation du TLS GitHub.

## HMAC concret et limites

Le protocole local signe METHOD, PATH complet incluant l'instance, TIMESTAMP et BODY exact. Le middleware sélectionne la clé de l'instance, compare en temps constant et refuse l'écart temporel absolu supérieur à30s. Le corps entrant est borné à64KiB avant parsing/client.

Le MAC ne chiffre pas le corps et sa fenêtre temporelle ne prouve pas le rejet de tout replay intérieur à la fenêtre. La consommation durable IAM protège une autre frontière : elle purge nonce et verifier au commit avant complete. Un Bearer utilisateur ne remplace pas ce HMAC ; Connect garde un AppState JWT refusant à JWKS vide.

## Rotation et diagnostic

Renouveler un certificat TLS signifie préparer le nouveau certificat/privé et son chargement, contrôler chaîne/nom et expiration. Changer une CA implique un rollout de confiance pouvant demander un chevauchement. Ce n'est pas renouveler le JWKS, tourner un secret HMAC ou changer l'issuer.

Une erreur CA/SAN se traite avant une hypothèse de mauvais `kid`. Un refus HMAC demande PATH/corps/clé/horloge exacts, sans les exposer dans les logs. Un token signé refusé demande issuer/audience/nonce/temps/algorithme, pas une désactivation globale de TLS. L'opérateur, le calendrier et les certificats réels restent à identifier ; aucun runbook n'est exécuté ici.

## Sources et navigation

- [Guide complet : handshake, frontières, FAQ et checklist](../../../../../docs/guides/iam-oauthconnect-securite-clefs-jwks-tls.md).
- [Sources S07–S10 et S15–S17 / RFC8446,9525,5280,9325](../../../../../docs/guides/iam-oauthconnect-securite-sources.md).
- [[projects/oauthconnect/concepts/oidc-jwks-and-key-rotation]] ; [[projects/iamrusty/concepts/oauthconnect-v2-durable-context]].
- [[projects/oauthconnect/oauthconnect]] ; [atlas des schémas](../../../../../docs/diagrams/README.md).
