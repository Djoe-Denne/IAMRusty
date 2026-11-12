# Trois scénarios k6 natifs — résultat local du 12 novembre 2026

## Résultat

**PASS local**, run `e3337dff24eb470881233fdfad6f6022`, sur
`kind-aiforall-local-full`. Windows natif, k6 2.3.0, Chromium for Testing
156.0.8078.73. Pas de build/test Rust dans Docker ni modification métier Rust.

- Santé : un GET HTTPS,200 et corps exact `OK`,2 checks nommés réussis.
- SAN : good303 → bad503 → restored303,2 checks par phase, sans suivre les
  redirects. Refus SAN+1 sous la même CA, même incarnation Envoy, restauration
  du certificat bon et retour fonctionnel vérifiés avant suppression.
- OAuth navigateur : **26/26 checks**, première inscription puis complétion,
  nouveau login, relink authentifié et trois refus403. Vrais IAM/OAuthConnect v2,
  HMAC par instance, profil IdP protocolaire, PKCE et passage ext-authz.
- Corrélation parent : IAM+7, ext-authz+4, fournisseur+3 et PKCE+3 ; JWKS fixture
  compatible avec le nominal. DB dédiée : users0→1, outbox0→0.
- **Cleanup PASS** :18/18 ressources reçues puis retirées par UID, DB éphémère
  supprimée après arrêt des writers, trois forwards joints, tous les processus
  k6/Chromium observés et terminés. Aucune rétention de fixture.

Les sources exécutées du parent/scénarios/fixtures correspondent à leurs
empreintes du reçu de préparation. La DB nominale n'est pas une cible d'écriture.
Un premier run complet `9f9a499f1789411a9826b21bed713d1e` avait également passé
les trois scénarios ; le run final ajoute l'oracle DB users/outbox et le cleanup
sans `kubectl wait` sur les Secrets.

## Corrections réellement nécessaires

1. Après reboot utilisateur : IDs Kind inchangés et API accessible, mais6/13
   actifs disponibles. Reprise native Bao/SQS, puis13/13 ; aucun reset, WinNAT,
   rebuild, suppression de nœud/volume ou activation des connecteurs nominaux0.
2. CA publique autorisée **de manière permanente dans CurrentUser\Root**,
   installation séparée via `trust-ca.ps1`. La campagne ne possède ni ne retire
   cette confiance. Empreinte DER SHA256 :
   `4a5256c189c6203f4579a383fb3d9104d84b479d94aee746c8fbb2e6d9a566e5`.
   LocalMachine inchangé ; aucune clé privée importée. Plus d'attente humaine
   consommant les dix minutes des forwards. Listeners possédés vérifiés avant
   stimulus ; raison/fin des forwards persistées.
3. Le cache Chromium historique ne contenait que chrome.exe :304 fichiers
   compagnons manquants. Extraction complète des305 fichiers depuis l'archive
   locale épinglée, sans téléchargement ni changement du Chrome habituel.
   Exécutable, manifeste de bundle et fichiers compagnons vérifiés avant usage.
4. Le tag k6 `group` est nécessaire aux checks nommés du projecteur. Une
   reproduction réelle sans réseau a montré leur absence sans ce tag, en mode
   compact comme full. Régression avec pass **et fail**, pas un résumé simulé.
5. Le cookie HTTPS produit est `__Host-iam-oauth-browser`, Path=/, Secure,
   HttpOnly, SameSite=Lax et Max-Age600. L'IdP public est donc
   `https://localhost:18445`, distinct du host IAM `127.0.0.1:18443` : aucun cookie
   IAM transmis au fournisseur. Politique produit inchangée. Lecture des cookies
   du contexte puis contrôle explicite nom/domaine/Path, car le filtre d'URL k6
   écartait le cookie racine. Conversion String du SameSite typé, égalité exacte
   `Lax` conservée ; headers HTTP contrôlés indépendamment.

## Validations et preuves

- Suite k6 :203 tests, **195 réussis**,8 tests POSIX non applicables sous Windows.
- Simulateur IdP :9 tests réussis.
- Trois scénarios réels, cinq phases : tous exécutés, aucun scénario sauté.
- `git diff --check` et syntaxe PowerShell ; rendu IaC positif et négatifs.
- Artefact public expurgé :
  `docs/missions/20261112-k6-three-native-evidence.json`.
- Originaux et reçus privés : dossier explicite `k6-native-20261112-i` sous le
  répertoire privé opérateur AIForAll, hors Git. Ne jamais l'archiver récursivement.
  Les empreintes de l'artefact public concernent uniquement ses projections
  allowlistées, pas les configurations/credentials/headers/dumps Admin bruts.

Les tentatives antérieures ne sont pas réécrites en PASS : run12-c avait une
fermeture ressources/DB réussie mais une identité de processus non prouvée lors
du refus SxS. Ce défaut est désormais propagé au verdict cleanup ; les runs
complets finaux ont un roster processus complet et zéro actif.

## Limites et disposition

- La preuve porte sur la **slice isolée** IAM/Envoy/OAuthConnect/IdP et les
  backings déclarés, pas sur la disponibilité de l'edge nominal ni la production.
  IAM conserve le mode opérateur `local_insecure`, sans bypass TLS des essais.
- DROP PostgreSQL contrôlé par reçu OID, zéro connexion et verrou advisory,
  sous la fenêtre DDL exclusive acceptée : contrôle puis DROP par nom ne sont
  **pas atomiques**. Ni FORCE, purge de queues, reset ni terminaison de session.
- Lab conservé en marche à la demande utilisateur :13/13 actifs, zéro Deployment
  fixture au contrôle final. Docker/WSL et leur RAM ne sont pas arrêtés/libérés.
- CI GitHub explicitement reportée ; aucun workflow réactivé ni secret créé.
- Les consommateurs legacy restent disponibles pour le moment : le lot publié
  qualifie le chemin local natif, sans prétendre avoir retiré tout le framework
  historique ni porté ce chemin vers Linux/CI.
- Contrat d'implémentation toujours **Proposed**, pas de promotion implicite ADR.

Sources : [cookies k6](https://grafana.com/docs/k6/latest/javascript-api/k6-browser/browsercontext/cookies/),
[implémentation k6 v2.3.0](https://github.com/grafana/k6/blob/v2.3.0/internal/js/modules/k6/browser/common/browser_context.go),
`services/IAMRusty/http/src/oauth_browser.rs` et les sources/reçus du parent natif.
