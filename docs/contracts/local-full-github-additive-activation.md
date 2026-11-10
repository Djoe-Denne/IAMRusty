# Local-full — activation additive GitHub, GitLab conservé

> Raccord Ops du 2026-11-09 : les chemins de l'ancien overlay sont historiques à `5199613`. Les déclarations et helpers publics résident maintenant dans le chart, voir le [guide](../guides/ops-declarative-boundaries.md). Seeds CAS, préimages et critères d'activation restent inchangés ; aucune activation n'est déduite de la migration.

2026-11-03 — **Proposed / Partial** pour le raccord décrit ici. Base
inspectée `7fdb17d33601ce85995fac8f1d89bef68d3c6b97`. Le livrable initial était
documentaire ; le contrat seul ne constitue ni autorisation runtime ni preuve E2E.

**Photographie du 2026-11-04 :** première tranche de code intégrée depuis la base
`1e89ff58143aaa59342d266d2611fd70036f43f4` : sélection du document IAM réellement
monté, composition privée GitLab+GitHub conservatrice, snapshot suivant ce volume,
bundle CA GitLab conservé et lectures privées revalidées. Les régressions initiales
OpenBao et leurs compléments passent (19 tests), ainsi que l'environnement (26)
et les profils (2, preuve antérieure réutilisée),
sans modification des tests par l'auteur de production. Le module d'alignement
navigateur passe 12 tests après correction indépendante de sa seule fixture :
gardes du renderer et assertions inchangées. **L'activation initiale, l'état pending/active, ses ACK et
la promotion ready ne sont pas livrés.** L'identité courante du Secret source et
la qualification de la postimage snapshot restent bloquantes ; aucun Secret GET,
déploiement ou lancement E2E n'est déduit de cette tranche. Les sections ci-dessous
conservent leur statut de contrat cible, pas de description d'une activation faite.

Complément à arbitrer : [qualification IAM initiale et post-snapshot](local-full-iam-preimage-qualification.md),
**Proposed / Partial**. Accord utilisateur local reçu le 2026-11-04 pour
la seule exception nommée du §3 de ce complément ; méthode privée d'ancre intégrée,
sans émetteur d'autorité livré. Ses 76 tests passent après correction de la chaîne
d'exception et relecture de sécurité ciblée PASS sur `572f3cd`, hors futur émetteur
et activation. Le refus
générique Secret GET est inchangé ; la méthode sans autorité refuse avant sonde.
Ni ce code ni ses 56 tests historiques ne qualifient l'activation ou le snapshot.

## 1. Autorité, décision et sources

Recommandation : opération spécialisée d'activation sur le **full existant**,
sous son bail original ; jamais `deploy` global, nouveau cluster ou remplacement
du catalogue par le profil GitHub seul. Elle ajoute GitHub au registre IAM
effectif, conserve GitLab, puis rend la reprise d'epoch éligible.

- Décisions **Accepted / Implemented** : [0407](../adr/0407-contrat-authn-federee-vendor-neutral.md),
  [0408](../adr/0408-connecteurs-idp-services-http.md),
  [0409](../adr/0409-confiance-callback-oauth-idp-connect.md),
  [0410](../adr/0410-migration-iam-connecteurs-idp.md),
  [0411](../adr/0411-idp-provider-slug-registry-fail-closed.md),
  [0412](../adr/0412-oauth-transaction-persistante-liee-navigateur.md).
  Catalogue au boot, doublons case-fold refusés, callbacks IAM et HMAC/HTTPS
  inchangés ; pas de hot-load ni de changement Rust.
- [Contrat SDK](startup-secrets-openbao.md) / ADR0505 : **Proposed / Implemented** ;
  [raccord OpenBao](openbao-local-githubconnect-deployment.md) : **Proposed / Partial**,
  avec GO de code borné historique, pas ratification globale.
  [Raccord E2E existant](local-full-existing-e2e-wiring.md) : autorité du bail,
  entrée attestée, inverse avant création ; sa garde Telegraph reste intacte.
- Handoff seed `.cursor/handoffs/20261103-github-kv-e2e-attempt.md` (ledger local
  du checkout principal, non versionné dans ce worktree) : seed
  terminé, GitLab seul effectif, `github_input` absent du registre d'environnement.
  Faits historiques transmis et relus, **pas remesurés live** dans cette mission.

Confrontation aux sources, chemins relatifs à la racine :

- `ops/deploy/local_full_openbao.py::configure` (71–165) exige une occurrence
  textuelle `connectors = []`, cherche un ConfigMap et produit GitHub seul.
  Ce n'est **pas** un inverse ni une migration du GitLab effectif.
- `ops/deploy/render-local-full.py::render` (132–197) interdit actuellement
  `github_openbao && oauth_proof` ; `proof_profile` (41–83) matérialise IAM dans
  un Secret à suffixe de run. `oauth-proof/iam.toml` contient GitLab avec les
  deux redirects, PKCE et les paramètres IAM à conserver. Lever seulement
  l'interdiction des deux flags serait insuffisant et dangereux.
- `ops/deploy/deploy-local-full.py::github_inputs` (169–198) : JSON fermé v1,
  doublons JSON refusés, entrée privée non symlink ≤64 KiB ; variante allowlist
  d'environnement non exécutée (les autres lignes y sont ignorées). Ne pas
  présenter cette variante comme un parseur dotenv intégral strict.
- `ops/deploy/local_full_environment.py::{input_reference,_register,operate}` :
  référence privée path/size/mtime, pas hash secret ; source ajoutée après
  inscription actuellement refusée comme changement. `recover_coffer` (66–131)
  exige Deployment GitHub déjà possédé, client ID concordant et init identities
  gérées. Il refuse explicitement les anciens workloads : **activation initiale
  distincte de recovery**, jamais suppression de ces refus.
- `ops/deploy/apps/overlays/local-full/helpers/openbao-workload.py::{seed,snapshot}` :
  seed des deux documents CAS0, comparaison privée, snapshot IAM du HMAC ;
  `local_full_environment.py::resync_api_policies` fournit l'analogue UID/RV/spec
  CAS, sans constituer une autorisation d'autres patches.

**ADR : aucune nouvelle.** Précision Ops locale réversible des contrats déjà
autorisés, sans API produit, protocole, stockage externe ou droit nouveau.
Le journal local privé proposé ci-dessous précise l'état d'une opération existante,
pas un nouveau service de stockage. Nouveau transport de secrets, lecture Secret
GET hors exception locale du complément de qualification, rotation,
élargissement réseau ou protocole durable ⇒ STOP et arbitrage
forward approprié. Aucun digest Serena requis sans écriture ADR. Le lead possède
le lien entrant depuis l'index ADR et les références communes, non modifiés ici.

## 2. Source unique du registre combiné

La **préimage est le fichier IAM privé réellement sélectionné/monté**, non le
template base à registre vide, ni une liste providers soumise par l'opérateur.
Qualifier sa provenance depuis le déploiement d'origine, son lien au Secret
monté/UID et au Deployment IAM du bail, et l'égalité de configuration effective
par un contrôle trusted expurgé. Secret GET interdit hors de l'exception locale
nommée du complément de qualification et de son admission native ; aucun export
de configuration Pod, dump base64 ou copie du HMAC GitLab dans un artefact public.

La source privée IAM d'origine doit donc être disponible et liée au montage
courant ; la projection publique historique SHA/providers ne remplace pas cette
source. Si seul le template avec markers ou un hash est disponible : **BLOCKED
SOURCE_IAM_UNAVAILABLE**, pas génération de nouveaux secrets GitLab/OAuthState.
Une couture de comparaison/rendu qui nécessiterait de nouveaux droits doit être
soumise au superviseur avant code ; ce contrat n'en accorde aucun.

Composer en mémoire une copie intégrale de la préimage, ajouter **une seule**
table GitHub au bon niveau TOML et parser avant publication. Un simple append
textuel n'est pas qualifié : les sections après `[[idp.connectors]]` et les
markers doivent conserver leur sens. Vérifier structurellement :

- un seul document IAM sélectionné et un seul registry ; toutes les lignes
  complètes/valides selon 0411 ; doublons après case-fold, IDs illégaux,
  sections ambiguës, markers inattendus ou données divergentes ⇒ refus ;
- préimage ciblée = GitLab seul ; tout autre profil non qualifié ⇒ refus. Une
  réinvocation avec GitHub n'est NOOP qu'avec reçu d'activation ACKé, mêmes UID,
  source et postimage exactes ; GitHub présent sans ce reçu n'est jamais adopté ;
- GitLab entier conservé (base URL, HMAC privé, PKCE, callback **et relink**,
  ordre/valeurs des redirects). Tous les paramètres IAM hors ajout GitHub restent
  sémantiquement identiques, notamment OAuthState, issuer/audience/JWKS, DB,
  files, TLS et chemins de clés. Pas de rerender à nouveaux secrets de proof ;
- GitHub : URL interne HTTPS8443 actuelle, PKCE false et les deux redirects IAM
  publics fixes du profil existant, exactement concordants avec Connect ; HMAC
  = marker unique réservé au snapshot KV, jamais valeur privée versionnée.

Le template combiné contient toujours le HMAC GitLab existant : il reste une
**configuration privée**, jamais un ConfigMap public. Réutiliser le transport
privé autorisé du déploiement et le snapshot IAM `emptyDir Memory` ; pas de
reclassification en non-secret parce que seul GitHub utilise un marker.
Ne pas modifier Envoy pour obtenir artificiellement le bon profil ; préserver
les routes GitLab callback/relink et les routes GitHub déjà qualifiées. Route
nécessaire absente ⇒ exposer un delta distinct avant activation.

## 3. Préflight et activation bornée (interface proposée)

Future interface spécialisée `activate_github_existing` dans le raccord Ops,
nom proposé, **pas CLI livré** : reçoit références privées, environment/bail et
images déjà publiées ; ni cible libre ni manifeste/commande/patch libre.
Le parent publie explicitement le consommateur après intégration/revue.

Avant toute mutation, sous `runtime.operation_lock` du bail original :

1. Résoudre le conflit **bloquant** : `.opencode/agents/infra-verifier.md:32`
   n'autorise que `kind-aiforall-local`, alors que la cible est local-full.
   Arbitrage humain/politique explicite et mécanisme de vérification indépendant
   réellement autorisé requis **avant activation**. Un rapport historique ou un
   autre profil improvisé ne remplace pas le verifier obligatoire.
2. Revalider ancre complète, namespace UID, epoch et bootstrap-ready OpenBao,
   scripts/helpers égaux aux sources revues, CA/SAN HTTPS, identités non-root,
   source privée IAM, source GitHub et images immuables publiées. Aucun build,
   extension PKI, bootstrap du coffre, root neuf ou rollout global caché.
3. Calculer une allowlist exacte des changements nécessaires IAM/GitHub et de
   leurs inverses. Préflight **tous** les UID/RV/spec/montages avant le premier
   écrit ; ressources inconnues/anciennes non qualifiées ⇒ refus. GitLab,
   ingress ACKé, Lazaret, Transit, PVC, certificats et données restent hors delta.
4. Persister intention privée liée à run/HEAD/cible/bail/epoch, références et
   préconditions non secrètes ; état non-ready avant création. L'extension de
   l'état environnemental reste rétrocompatible et fermée : phase, UID ACKés,
   epoch visée et références seulement, pas valeurs/hash OAuth/HMAC. Écriture
   atomique privée avec contrôle de préimage ; lock du bail + refus si l'état
   a changé. Un fichier d'état n'accorde jamais la propriété runtime.

Ordre ensuite : enregistrer la référence GitHub comme **pending**, valider les
fondations déjà possédées, seed nonroot CAS0 idempotent si nécessaire, créer
seulement les objets GitHub absents autorisés (create-only avec ACK UID), préparer
le template privé combiné et ses identités, démarrer Connect puis basculer IAM
par patch borné UID/RV/préimage. Ne pas appeler `deploy_locked`/apply global.
Conserver le template préimage privé sans en publier le contenu ; nouvelle
configuration privée ne remplace aucune source étrangère au bail.

Après chaque écrit : ACK UID et postcondition au reçu avant phase suivante.
Rollout IAM requis par 0411, aucune attente de hot-load. Promotion de la référence
`github_input` en active et état ready seulement après vérification courante de
toutes les postconditions : GitHub **et** GitLab effectifs, bon montage/UID,
Connect bon client ID/HMAC issu du même KV, GitLab redirects inchangés,
HTTPS/CA stables, epoch stable, reprise qualifiée et verifier indépendant PASS
sur le delta. TCP Ready seul ne prouve ni HMAC ni conservation des paramètres.

## 4. Sources privées, ressources ACKées et reprise

`github_input` conserve le format existant `input_reference` : chemin canonique,
size et mtime_ns, accès privé/read-only, puis validation par `github_inputs`.
Le lier explicitement au bail/environment, client ID public et profil activé ;
ne pas remplacer silencieusement un `github_input` déjà inscrit, même absent sur
disque. Source absente, symlink, ACL ouverte, changée, mauvais client ID ou autre
bail ⇒ refus **avant seed/patch**, diagnostics constants expurgés. Revalider la
référence avant/après lecture et avant usage ; modification concurrente ⇒ STOP.
Path/size/mtime sont un détecteur de changement, **pas une preuve cryptographique
d'immuabilité** : garantir la lecture cohérente owner-only pendant l'opération,
et ne pas prétendre détecter un attaquant trusted réécrivant les mêmes métadonnées.
Ce modèle de confiance n'autorise aucune empreinte de valeur secrète en preuve.

Pour le seed déjà réalisé, le handoff atteste sous le bail original :

- SA `github-seed` UID `884951e2-f86e-4ec4-8e56-f05204b37934` ;
- NP `allow-openbao-from-github-seed` UID `9d1e770d-31d2-4a0c-b040-a1522e64e01f` ;
- NP `allow-github-seed-to-openbao` UID `8595000a-aca6-4496-a417-b3873400ce9b`.

Ce sont des contraintes historiques à confronter aux ACK du bail et aux specs
revues, pas une nouvelle autorité en dur. UID remplacé, spec changée, ACK absent
ou bail différent ⇒ STOP. Ne pas recréer/adopter par nom, ni ajouter une seconde
SA/NP redondante ; les deux NP8443 sont conservées comme setup seed. Job TTL/Secret
historique n'est pas ressuscité ; absence du Secret inférée du GC n'est pas preuve
d'effacement etcd. Nouveau seed : Job exclusif, Secret ownerReference **UID du
Job** et transfert mémoire/stdin existants, aucune valeur dans argv/rendu/log.

Reprise après activation : `recover_coffer` conserve ses préconditions managed
identities, source inchangée et Deployment GitHub possédé ; il ne sert jamais
à effectuer le premier upgrade IAM. Même epoch saine : pas de reseed/rotation
pour commodité. Nouvel epoch : bootstrap privé terminé et root révoqué selon le
mécanisme existant, `pending_epoch` persisté, deux entrées validées avant seed,
CAS0 seulement si absence réelle ; identiques = zéro nouvelle version,
différentes/supprimées/détruites/403/transport KO = refus, jamais « absent ».
Puis snapshots/agent et rollouts ciblés IAM/Connect/Lazaret selon recovery actuel,
sans toucher GitLab ni les autres workloads. L'activation n'impose pas un restart
du coffre pour s'épargner la comparaison des valeurs déjà seedées.

Interruption, ACK perdu ou conflit CAS : non-ready, état privé conservé, pas
retry aveugle, adoption par GET ou rollback destructif des KV. Les deux documents
ne sont pas transactionnels ; une écriture réussie reste conservée. Reprendre
uniquement la même tentative avec ACK et postconditions démontrés ; Job disparu
avant son ACK de réussite ⇒ état incertain, arbitrage, pas recréation automatique.
Epoch nouvelle pendant reprise ⇒ conserver `pending_epoch` et refuser comme
`running_operation` actuel. Ne jamais publier epoch achevée après reprise partielle.
Rollback IAM seulement si sa postimage propre et UID/RV courants sont prouvés ;
sinon arrêt explicite. Aucune compensation ne détruit les ressources étrangères.

## 5. Lot implementer, TDD et admission

Impact **cross-module / contrat Ops et trust local** : `local_full_openbao.py`
(composition structurée privée, pas substitution empty-only),
`deploy-local-full.py` (opération ciblée et validation des références),
`local_full_environment.py` (inscription additive pending/active et reprise),
`render-local-full.py` seulement pour une composition offline explicitement
qualifiée ; ne pas changer le comportement historique des deux flags par défaut.
`helpers/openbao-workload.py::snapshot` doit accepter le template privé combiné
sans exposer GitLab ; toute modification y reste locale et sous revue secrets.
Pas de migration DB, Rust/SDK, CA/PKI, nouveaux droits ou framework.

Le lead attribue **avant le lot code** production à implementer et tests du
contrat QA à **test-engineer**, en périmètres isolés ; aucune fiche différée ne
transforme implicitement ce contrat en ownership production/tests unique.
Régression ROUGE discriminante avant correction quand possible : composition
GitLab non vide refusée actuellement, ajout initial de `github_input` refusé,
recovery ancien IAM refusé. Un prérequis live absent n'est pas une régression rouge.

Tests hermétiques attendus (fakes, aucun accès aux vrais secrets/runtime) :

- `ops/tests/test_openbao_local.py` : registre GitLab non vide → exactement
  GitLab+GitHub, égalité des paramètres/redirects ; case-fold duplicate,
  configuration ambiguë, marker invalide, source IAM indisponible refusés sans
  écrit ; seed identique conserve versions, CAS perdant et premier document seul
  écrit restent non-ready ; sentinelles OAuth/HMAC absentes de tous diagnostics.
- `ops/tests/test_local_full_environment.py` : inscription pending initiale puis
  promotion, réinvocation propre, source absente/modifiée/étrangère, faux client ID,
  mauvais UID SA/NP/Deployment, conflit état/CAS avant et pendant mutation ;
  ACK perdu, crash entre create/ACK et entre seed/rollouts, même epoch et deuxième
  epoch interrompant la première. Refus sans adoption ni écrasement de GitLab.
- Si diff de sélection/routage effectif : compléter les tests existants
  `test_local_full_route_profiles.py` et `test_oauth_browser_alignment_source.py`
  pour les deux callbacks/relinks conservés ; ne pas exécuter sans diff concerné.

Commandes du futur raccord, depuis la racine, uniquement après son admission :

```text
python -m unittest discover -s ops/tests -p test_openbao_local.py
python -m unittest discover -s ops/tests -p test_local_full_environment.py
```

Selon diff de routage, ajouter les mêmes commandes `unittest discover` pour
`test_local_full_route_profiles.py` et `test_oauth_browser_alignment_source.py`.
`git diff --check` toujours. **Ici : revue sources et contrôle documentaire
seulement**, aucun Python applicatif/Cargo/Docker/Kind/live nécessaire.
Revue indépendante bornée secrets/trust obligatoire si le futur diff modifie
cette frontière. Activation et qualification restent gated par le conflit verifier.

L'activation réussie ne lance pas automatiquement les E2E : GitHub/GitLab effectifs
ne lèvent ni la garde Telegraph avant signup ni les autres préconditions. Garder
39 obligations mesh, 6 Apparatus et `SIMULATED_PROVIDER` ; aucun PASS global ni
login OAuth vendor déduit d'un seed, d'un boot ou d'un PID.
