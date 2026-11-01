# OpenBao / GitHubConnect — livraison d'implémentation

## Portée et autorisations

Suite de [la mission préparée](20261031-openbao-local-github-connect.md), base `678d22f`, SDK consommé `93b890f` inchangé. GO utilisateur pour le code, les vérifications offline/natives, la fenêtre Lazaret de 8 h et ses correctifs de confidentialité/CA. Autorisation complémentaire explicite du petit transport HTTPS/CA P4 : son ancien login utilisait un `TcpStream` exclusivement HTTP. Aucune migration SDK/Transit, aucun changement de clés/policies P4 métier.

**L'implémentation ne vaut pas qualification du déploiement.** Docker/Kind, images Linux, appels Kubernetes et credentials réels ne sont pas lancés par cette phase. Les images concernées restent à construire/attester après publication du code, puis le runtime à qualifier avec des sentinelles synthétiques.

## Mécanismes livrés dans l'arbre

- Un seul bootstrap hôte Python pour Windows/Linux, façade PowerShell conservée : même autorité leasev2, mêmes contrôles CNI/CIDR, Calico 3.29.7, pool `10.244.0.0/16`, FELIX, CoreDNS et nœuds Ready. Aucun contournement par un shell Linux.
- OpenBao local volatile : root CSPRNG neuf dans la frontière serveur/bootstrap, bannière et logs DEV supprimés avant le runtime, boucle privilégiée bornée, root révoqué et refus vérifié avant readiness. HTTP uniquement dans le loopback du coffre ; clients inter-Pods en HTTPS 8443.
- Deux leaves ajoutées dans `extensions/openbao-github/`, option explicite `--openbao-github`. Les anciennes CA/leaves/séries ne sont pas modifiées ; bundle partiel refusé. Aucun nouveau partage de clé CA ou de clé serveur via le sac mesh existant.
- Lazaret : SA/role propres, lecture du seul document `secret/data/ops/token`, bearer 8 h sans renouvellement ; TOML privé en `emptyDir Memory` rendu au login init. Debug expurgé, token non sérialisé, CA explicite, aucun fallback root. **Avant 7 h 45 : nouveau Pod/login** pour poursuivre ; restart du seul conteneur ne renouvelle pas ce snapshot. Une nouvelle epoch du coffre exige aussi ce renouvellement explicite.
- GitHub : profil `--github-openbao`, distinct et exclusif du `--oauth-proof` GitLab. OpenBao Agent durable en sidecar native, JWT projeté seulement dans l'agent, token-file 0600 en RAM lu par le SDK, TTL initiale 15 min / plafond 1 h. La permission `renew-self` n'est pas ajoutée ; la réauthentification et le restart tardif restent à prouver sur le pin effectif.
- Seed GitHub distinct après le bootstrap root : droits limités aux deux documents et leurs métadonnées, comparaisons privées, CAS=0, refus des conflits/suppressions/destructions, pas de rollback destructif. Secret d'entrée créé exclusivement, lié à l'UID du Job ; GC par TTL du Job après terminaison. **Copie dans etcd possible**, sans promesse d'effacement des sauvegardes ni de RAM-only du système entier.
- IAM reçoit le même HMAC depuis le coffre via un init HTTPS, dans un TOML privé en RAM, sans changement de son code. Snapshot seulement ; toute rotation future exige un rollout coordonné IAM/Connect.
- Les chemins de livraison d'images sont ciblés : profil d'artefacts `openbao` pour GitHub/Lazaret/migration Lazaret ; images P4 compilées explicitement. Un simple repackage d'artefacts anciens n'est pas une preuve du nouveau service/SDK. Les labels de provenance sont des métadonnées de build, pas des signatures de supply chain.

Pour la future construction Linux (non exécutée ici), utiliser un checkout de livraison propre au commit publié et le template de contexte dédié, sans `.env`. Fournir `DELIVERY_PROFILE=openbao`, `SOURCE_REVISION=<commit des sources>` et `SDK_REVISION=<gitlink consommé>` au `Dockerfile.build`. Reconditionner ensuite **seulement** GitHub/Lazaret avec ce `BUILD_IMAGE` et les mêmes révisions ; le packaging attesté refuse un profil différent ou des métadonnées discordantes. Les Dockerfiles P4 construisent explicitement leur seul binaire, avec les mêmes labels de source/SDK. Construire séquentiellement (slot Cargo) et enregistrer les IDs créés à la création selon le registre du dépôt. Le mapping local et le déploiement refusent ensuite une image sans provenance correspondante ou dont les sources pertinentes diffèrent de l'arbre conservé.

## Utilisation préparée — runtime non exécuté

Préparer ou étendre les certificats hors dépôt :

```powershell
.\ops\scripts\prepare-local-full-certs.ps1 -OutputDirectory "$HOME\.aiforall-local-full-certs" -OpenBaoGitHub
```

Linux : `python3 ops/scripts/prepare-local-full-certs.py --output-dir "$HOME/.aiforall-local-full-certs" --openbao-github` ; `--oauth-proof` reste une option séparée pour créer dès l'origine le superset GitLab.

Le fichier privé fourni par `--github-input-file` est soit un JSON de schéma exact `version=1`, `client_id`, `client_secret`, `hmac_secret`, soit un `.env` lu par allowlist : `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_IAM_HMAC_SECRET`. Les autres clés ne sont pas transférées ; aucun sourcing/expansion shell. HMAC : 64 caractères hexadécimaux minuscules. Le fichier doit être privé, en lecture seule, dans un répertoire privé ; jamais un argument contenant une valeur sensible, ni un fichier du contexte de build.

Entrée de déploiement, après acquisition explicite du bail/cible et livraison d'images immuables :

```text
python ops/deploy/deploy-local-full.py deploy --lease <bail-prive> --image-manifest <mapping-immuable> --cert-directory <certificats-prives> --github-openbao --github-input-file <entrees-privees>
```

Rendu offline sans aucune entrée secrète : `python ops/deploy/deploy-local-full.py render --github-openbao`.

Le contexte reste **local-full uniquement** ; aucune cible prod/cloud n'est ajoutée. Le profil smoke n'autorise pas un egress Internet général : l'egress vers le vrai fournisseur GitHub reste un gate explicite du futur parcours OAuth réel. Rien ici ne constitue une connexion utilisateur GitHub qualifiée. Ne pas utiliser de vrais credentials avant la qualification du durcissement avec sentinelles et l'autorisation correspondante.

## Vérifications et gates

Preuves Windows acquises sans Docker/Kind :

- `cargo check -p lazaret-setup -p apparatus-operator --features apparatus-operator/controller` : PASS.
- Régression native `cargo test -p lazaret-configuration -p lazaret-infra -p apparatus-operator --features apparatus-operator/admit --lib` : **50 tests PASS** (3 + 21 + 26), dont le TLS réel loopback P4 bonne CA/CA étrangère/mauvais SAN et l'expurgation du token Lazaret.
- `test_openbao_local.py` : **6 PASS**, dont génération réelle d'une PKI synthétique, ajout/réutilisation non destructifs, refus d'un bundle partiel, garde de provenance et parité du bootstrap CNI. Aucune lecture des certificats personnels.
- Garde cible `test_local_full_target.py` : **52 PASS** ; identité serveur Envoy : **4 PASS** ; alignement OAuth navigateur : **12 PASS**, avec la fixture de filtrage de routes isolée du nouveau wiring de coffre.
- Avertissements existants SDK (`async_fn_in_trait`) et documentation P4 conservés, pas de prétention Clippy global vert.

La CI Linux reprend le même test offline ; son exécution effective reste à constater après publication. Les tests de modèle CAS et les rendus ne prouvent pas un coffre réel. Le transport TLS loopback P4 vérifie effectivement le client modifié, pas l'image OpenBao. Les snapshots IAM/Lazaret utilisent le helper Python HTTPS de l'image init déjà épinglée, plutôt qu'un second moteur de templates Agent ; l'Agent durable reste réservé au token-file GitHub.

Validation finale du code figé : **124 tests PASS** (50 Rust + 74 offline/source), syntaxe des 10 fichiers Python, 7 YAML, 3 scripts shell et 2 façades PowerShell modifiés, fmt ciblé et diff whitespace PASS. Les derniers ports YAML, contrôle UID et métadonnées de packaging étaient inclus dans cet index ; seules les précisions documentaires sont postérieures. Revue infra indépendante puis re-vérification ciblée : aucun finding statique bloquant restant. Le finding Cosign a été levé après vérification du binaire hérité dans le rendu réel ; trust store IAM et flux JWKS ont été corrigés. L'ancien helper gold HTTP non consommé reste hors lot, préservé avec les autres laboratoires. Verdict runtime **INCONCLUSIVE**, sans changement de sens des PASS offline.

Restent indispensables : bootstrap/revocation sur OpenBao 2.6.2 épinglé ; bon/mauvais SA/audience et moindre privilège ; TLS de chaque client effectif (dont IAM et Cosign) ; seed réentrant réel ; restart du seul conteneur Connect après expiration du premier bearer ; expiration/relogin Lazaret et nouvelle epoch du coffre ; inventaires et nettoyage des seules ressources possédées. Pas de PASS déduit d'un Pod Ready.
