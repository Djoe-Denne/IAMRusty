# Revue indépendante — raccord ops OAuthConnect

## Provenance et portée

Auteur : sous-agent `infra-verifier`, session `ses_e447e48acfferuDDDS3orLXEC6`,
enfant de `ses_e449ec10affeZ5b0fUHSKL1PZI`. Conclusion finale
`msg_1bb8ca83a001QBabfe3UvhQmJv`, créée le2026-11-08T14:25:19.933Z.
Source : projection visible API read-only, confirmée par le retour au parent.
Le texte ci-dessous est une distillation, pas un nouveau contrôle exécuté.

Base examinée : `8575620c...` et diff candidat, nouveaux fichiers inclus.
Mandat read-only statique/offline : aucun Docker/Kind/WSL, kubectl live, secret
réel, PKI opérateur, build Linux ou Cargo concurrent. Aucun fichier modifié.

## Verdict retenu

**PASS à portée statique/offline**, aucun blocage fonctionnel/configuration/TLS
détecté. **INCONCLUSIVE pour le live** ; le périmètre concorde avec ADR0413
Accepted/Partial. Ce verdict n'est pas converti en preuve de déploiement.

## Vérifications positives

- **Profils** : exclusivité des profils legacy GitHub/GitLab et OAuthConnect
  dans le renderer, le consommateur d'images et le déployeur.
- **Origine et identité** : `PUBLIC_ORIGIN=https://127.0.0.1:18080`, callbacks,
  public_base_url/issuer IAM et consommateurs cohérents ; assertions d'égalité
  sémantique de la préimage IAM conservant les paramètres non concernés.
- **TLS** : identité Envoy exacte exigée, exigence de certificat client retirée
  seulement sur l'entrée navigateur ; tous les transport_socket upstream
  conservés. SAN IP déjà présent dans la PKI et mapping18080→30080 existant.
- **Route relink** : copie exacte GET/HEAD GitHub vers le seul chemin
  `simulated/relink-callback`, puis filtrage des callbacks non configurés.
- **PKI** : extension `openbao-oauthconnect` distincte, mêmes racines admises,
  identités DNS Connect vérifiées, anciennes feuilles inchangées et bundle
  partiel refusé. Confiance IAM = système + CA transport explicite.
- **HMAC** : markers uniques, deux slugs attendus, secrets distincts et aucun
  publish si résultat invalide ; aucun accès OAuth vendor dans le rôle IAM.
- **OpenBao** : chemins exacts, rôles/SA/audiences cohérents, droits de lecture
  séparés et seed RW+metadata ; pas de policy wildcard/default implicite.
- **Bootstrap/reprise** : seed CAS0 sans écrasement, sources privées inchangées,
  vérification des préimages avant mutation, Job/Secret liés par UID, patches
  UID/resourceVersion des dépendants et compatibilité des états legacy.
- **Images** : cible `oauth`, package/binaire/ENTRYPOINT identiques, image de
  livraison non-root, digest base épinglé, source image et reçu cohérents,
  références immuables et politique de pull appropriée.
- **Contrat de service** : `/oauth-connect/ready` sur8443, dual-bind SDK,
  enums/config/ref KVv2 conformes, routeur v2 partagé et JSONPath pris en charge.
- **Legacy** : HTTP downstream, routes, configuration additive GitHub, seed
  GitHub et recovery antérieurs restent distincts. Tests historiques étendus,
  pas remplacés pour masquer une régression.

Localisation principale : `ops/deploy/{render-local-full.py,local_full_oauthconnect.py,
deploy-local-full.py,local_full_environment.py,image_catalog.py,local-full-images.py}`,
helpers OpenBao, PKI et tests sous `ops/tests/` ; configuration et Dockerfile
sous OAuthConnect. Les numéros de ligne du retour original décrivaient le diff
au moment de la revue, pas un invariant de localisation future.

## Observations non bloquantes

1. **F1 faible latent** : `render-local-full.py` utilise `next(...)` pour retrouver
   la route relink GitHub source. Une disparition future produirait un
   `StopIteration` non converti en erreur de domaine. La source actuelle la
   contient et le test la couvre ; aucune correction de cette dette n'est
   revendiquée par la session.
2. **F2 informationnel** :8080HTTP est aussi lié par le SDK ; seuls8443Service et
   ingress IAM→8443 sont déclarés. Ce n'est pas une exposition publique HTTP.
3. **F3 informationnel** : feuilles d'extension90jours, politique locale
   existante. Rotation non qualifiée ni exécutée ici.

## Ce qui reste inconnu

Image Linux construite, reçu réel chargé/pullé, rôles et login OpenBao effectifs,
seed/recovery sur coffre réel, validation par le binaire Envoy, handshake avec
CA navigateur, chaîne IAM→Connect v2, GitHub live et tunnel18081 du scénario
simulé. Les logs Python/Cargo consultés corroborent des tests offline, pas ces
faits live. Aucun ancien PASS de protocole n'est étendu à ces gates.

Voir le [distillat de session](README.md) et le
[rapport de mission](../../missions/20261108-ops-oauthconnect.md).
