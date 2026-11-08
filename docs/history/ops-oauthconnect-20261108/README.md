# Session ops OAuthConnect — distillation du 8 novembre 2026

## Couverture et frontière de confiance

Source : session racine `ses_e449ec10affeZ5b0fUHSKL1PZI` et son unique enfant
direct `ses_e447e48acfferuDDDS3orLXEC6` (`infra-verifier`). Capture API read-only
bornée à la demande d'ingestion, **2026-11-08T14:32:54.690Z**. Le catalogue des
enfants a été paginé jusqu'à épuisement ; aucun autre enfant direct observé.

- Racine : 138 messages avant frontière, lus sur 3 pages API ; 3 messages
  utilisateur et 12 parties de texte assistant retenus dans la projection privée.
- Revue : 37 messages sur 2 pages API ; 1 briefing utilisateur et 14 parties de
  texte assistant retenus dans la projection privée.
- Total : 175 messages examinés, 30 messages/parties visibles projetés ;
  198 parties de raisonnement et 262 parties d'outils exclues de la projection.
  Les messages système, métadonnées de modèle, sorties shell et notifications
  synthétiques ne sont pas publiés. Les choix de formulaire, observés pendant la
  session mais non représentés comme messages `user` par l'API, sont explicités
  ci-dessous sans publier les sorties d'outils.

Les projections privées restent hors Git dans `.tmp/ops-oauthconnect-wiki/`.
Ce dossier historique publie seulement le distillat, la revue et l'inventaire
non secret. Ce n'est ni un export brut, ni une nouvelle capture des 36 anciennes
feuilles du lot IAM/Connect précédent. Les messages ultérieurs d'ingestion sont
hors frontière ; les sessions supprimées ou non rattachées ne sont pas couvertes.
Voir [coverage.json](coverage.json).

## Demande et décisions utilisateur

L'utilisateur demande d'abord de lire code et documentation de la fusion avec
QMD et Rust Code MCP, puis d'adapter les ops pour lancer le service générique
avec GitHub et un fournisseur préparé pour les futurs E2E. Les sources du dépôt
nomment GitHubConnect/GitLabConnect ; le terme initial « Keydock Connect » n'a pas
été retrouvé et n'est pas transformé en nom de service dans les connaissances.

Décisions observées :

1. Instance `simulated`, profil **OAuth**, par réutilisation du simulateur
   existant ; pas d'ajout Discovery/JWKS/ID tokens OIDC signés maintenant.
2. Secrets GitHub existants, chaîne de distribution OpenBao conservée ; aucune
   valeur réelle lue ou placée dans une commande, configuration publique ou log.
3. Lorsque la configuration vérifiée refuse les callbacks HTTP locaux, la
   proposition d'exception loopback n'est pas approuvée. L'utilisateur répond :
   **« Ne serait-il pas plus simple d'avoir un HTTPS ? »**. La solution retenue
   est une entrée navigateur HTTPS, pas une désactivation de TLS GitHub ni un
   changement du validateur de configuration.
4. Les profils legacy restent séparés ; aucun retrait de service, migration de
   compte, bascule ou E2E fournisseur n'est exécuté dans le lot.

## Résultat durable et frontières techniques

Le profil explicite `--oauth-connect` traverse plan/build, reçu d'images,
rendu et déploiement. Il construit un seul catalogue GitHub+simulated dans le
vrai service OAuthConnect. Les autres profils sont incompatibles avec ce flag,
pas des fallbacks. La cible d'image `oauth` et son Dockerfile rejoignent la chaîne
existante de provenance source/SDK et de références immuables.

Entrée publique : `https://127.0.0.1:18080`. Le certificat Envoy existant porte
le SAN IP ; le navigateur n'a pas de certificat client à présenter. Les transports
mTLS upstream ne sont pas changés. Callbacks exacts, origine/cookie IAM et issuer
des consommateurs sont alignés. La confiance CA navigateur et l'enregistrement
du callback dans l'application GitHub existante restent des étapes opérateur.

GitHub reste en transport vérifié avec le scope `user:email`, le profil `/user`
et l'email primaire de `/user/emails`, sans fallback email public. `simulated`
possède autre slug/client/HMAC, PKCE S256 et booléen `email_verified`. Seul son hop
HTTP interne utilise `isolated_test`. L'URL d'autorisation loopback18081 exige le
tunnel ancré du futur scénario ; le profil ne l'ouvre pas implicitement.

Les chemins KV GitHub `github-connect/oauth` et `github-connect/iam` sont
conservés. Deux nouveaux documents `oauth-connect/simulated/{oauth,iam}` portent
les credentials dédiés. Préimages vérifiées toutes avant écriture, réutilisation
exacte ou création CAS0 ; aucun écrasement/effacement réparateur en cas de conflit.

Connect lit les quatre documents via Agent/token-file au boot ; IAM ne lit que
les deux HMAC ; le simulateur ne lit que son client secret. Snapshots privés en
Memory. Le Secret d'entrée seed est lié au Job par UID/TTL, ce qui n'en fait pas
un stockage RAM-only dans etcd. La reprise d'epoch exige les deux sources privées
inchangées et contrôle les UID/resourceVersion des seuls dépendants à redémarrer.

## Essais et corrections, sans effacer les échecs

- Le premier test Rust de la configuration réelle échoue sur les callbacks HTTP
  avec le profil fournisseur `verified`. Le passage HTTPS le ferme sans changer
  le validateur ni les attentes du test.
- Deux premiers oracles Python cherchaient un nom ConfigMap sans son suffixe
  Kustomize ; la localisation du vrai objet est corrigée, pas le contrat attendu.
- Une indentation Python puis des erreurs d'échappement de scripts d'analyse
  sont corrigées localement ; ce ne sont pas des pannes de MCP.
- Un wrapper PowerShell retourne1 sur redirection stderr native alors que les
  résultats libtest sont verts. Le journal est UTF-16LE ; décodage strict puis
  commande finale native `cmd /c` distinguent résultat de test et statut wrapper.
- La nouvelle rustdoc et le formatage des tests sont corrigés avant le Clippy
  strict et le hook normal, sans `allow`, réduction de jobs ou contournement.

Preuves retenues, sans sommer des campagnes différentes : suite Python complète
215 tests dont213réussites/2ignorés, puis18/18OAuthConnect et19/19OpenBao après
ajout de deux oracles et extension PKI. Les deux skips sont des preuves opt-in
cache Cargo/registre Docker. Configuration Rust39/39, puis3/3tests ops ciblant la
déclaration et les mappings réels. Clippy/fmt ciblés verts et hook fmt normal
passé ; avertissement SDK `async_fn_in_trait` préexistant conservé.

## Revue indépendante et limites

[Revue infra](infra-review.md), message final
`msg_1bb8ca83a001QBabfe3UvhQmJv` du **2026-11-08T14:25:19.933Z** :
**PASS statique/offline**, aucun blocage. Le live reste **INCONCLUSIVE**.
Le reviewer n'a modifié aucun fichier ni exécuté de runtime.

F1 faible latent : disparition future de la route relink GitHub source ⇒
`StopIteration` au rendu au lieu d'une erreur dédiée ; présence actuelle testée.
F2/F3 informatifs : port8080 lié par le SDK mais non exposé, validité des feuilles
90jours selon la politique locale existante. Aucun de ces constats ne devient
une promesse de runtime ou une correction silencieuse dans cette ingestion.

Non exécutés : build Linux, coffre/Agent réels, apply Kind, handshake navigateur,
confiance CA système, application GitHub enregistrée, flux GitHub live, E2E simulé
ou chaîne complète IAM→Connect→vendor. ADR0413 demeure Accepted/Partial ; gates
GitLab/sub/cutover et qualifications antérieures restent distincts.

## Publication et sources

Implémentation `f78108bec6288b52517abb51546d8a044001fc62`, clôture documentaire
`1866517e88ea40949574b2baa2e6da45d4b34bd1`, publiées sur `origin/master` vérifiée.
Unique checkout/master propre à cette clôture, SDK `93b890ff...` inchangé ; aucune
branche/worktree créé, aucune équipe à archiver et aucun runtime possédé.

- [Rapport livré](../../missions/20261108-ops-oauthconnect.md).
- [Guide opérateur](../../../ops/deploy/README.md).
- [Configuration publique utilisée](../../../ops/deploy/apps/overlays/local-full/oauth-connect/connect.toml).
- [Composition ops](../../../ops/deploy/local_full_oauthconnect.py),
  [reprise](../../../ops/deploy/local_full_environment.py) et
  [distribution OpenBao](../../../ops/deploy/apps/overlays/local-full/helpers/openbao-workload.py).
- [Revue distillée](infra-review.md) ; [inventaire borné](coverage.json).
