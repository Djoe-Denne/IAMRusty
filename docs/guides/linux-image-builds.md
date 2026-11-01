# Images Linux : itération locale et release

`ops/deploy/build/images.py` est l'entrée Python commune Windows/Linux pour
**planifier et construire** les images. Aucun déploiement, E2E, démarrage de
Kind, génération de certificat ou provisioning OpenBao n'est implicite.
Cette première tranche produit des **IDs Docker locaux**, pas encore des digests
de manifestes publiés dans Zot. Le Zot P4 et ses données ne sont pas modifiés.

## Développement

Plan sans Docker :

```sh
python ops/deploy/build/images.py plan --github-openbao
```

Construction lorsque le daemon Linux est disponible, avec le builder courant
de type `docker` (aucun changement global de builder/trust n'est effectué) :

```sh
python ops/deploy/build/images.py build --github-openbao --output image-build.json
```

- Les sources sont les **octets actuels** : modifications non committées,
  nouveaux modules/assets publics, même Git-ignorés, et SDK réellement patché
  sous `rustycog/`. L'inventaire est une allowlist de sources/recettes/assets,
  pas une copie récursive de tous les fichiers ignorés (bases/logs/caches exclus).
- Le contexte est une copie possédée et bornée aux sources/recettes publiques.
  `.env`, clés, dossiers privés et état agent sont exclus avant lecture/hash.
  Le `.dockerignore` de l'opérateur n'est jamais remplacé. Ne pas mettre de
  vrais secrets dans les fichiers de source/configuration publics.
- La compilation utilise le snapshot, jamais un mélange de fichiers vivants.
  Les différences restantes à la comparaison finale font échouer l'opération.
  Arbitrage utilisateur du 2026-11-01 : A→B→A est admissible si le snapshot
  compilé et toutes les entrées finales sont identiques A, SDK compris. Ces
  comparaisons ne prétendent pas détecter chaque écriture transitoire.
  Le reçu et les labels nomment cette politique `snapshot-final-v1`.
- Une image déjà conforme est réutilisée ; sinon un seul Cargo reçoit la
  sélection complète pour conserver une unification stable des features. Cargo
  réutilise les artefacts inchangés ; seules les images périmées sont emballées.
  La sélection et le profil sont inclus dans l'identité de construction.
  Un changement SDK invalide **tous** ses consommateurs
  sélectionnés, pas uniquement GitHub/Lazaret. Le déploiement refuse les autres
  images devenues périmées : un build partiel n'autorise pas leur réemploi.
- Les caches Linux registry/git/target restent persistants. Le cache target
  historique est conservé, protégé par `sharing=locked`. Les mtimes des sources
  sont gérées par empreinte pour détecter aussi un rollback de contenu. Le
  journal de synchronisation est remplacé atomiquement **avant Cargo**, même
  si la compilation échoue ensuite ; il n'atteste jamais un build réussi.
  Horloge en retrait ou précision insuffisante ⇒ refus, sans purge du cache.
- Le profil `dev-image` privilégie l'itération fonctionnelle. Pour les mesures
  de performances, utiliser `--profile release`. Les 24 jobs Cargo restent
  configurés dans `.cargo/config.toml` ; le target Windows E: n'est pas partagé.
- `--targets hive` borne explicitement le build. Les profils complets ajoutent
  GitHub avec `--github-openbao` ou GitLab avec `--oauth-proof` (distincts).

Compatibilité avec le chemin existant **kind load**, après obtention du bail :

```sh
python ops/deploy/local-full-images.py --lease LEASE --github-openbao --build-manifest image-build.json --output images.json
```

Le manifeste doit couvrir tous les consommateurs Rust du profil. Le Zot P4
préexistant reste une image séparée ; il n'est ni reconstruit ni adopté par ce
build. Les anciennes images attestées seulement par HEAD doivent être
reconstruites avant utilisation : HEAD seul ne prouve pas l'arbre courant.

## Release

**Non implémentée dans la tranche A.** `--policy release`, `--revision` et
`--cold` sont refusés avant tout build : export automatique du commit exact de
`master` et de son gitlink SDK, référence distante et pins complets des outils
restent à réaliser. Un checkout propre n'est pas un substitut à ce contrat.
L'export futur ne devra ni modifier ni exiger de nettoyer le workspace courant.

`--policy local --profile release` reste disponible pour des **mesures locales**,
pas pour déclarer une livraison cloud. Le namespace target neuf de compilation
froide reste une politique future ; aucune purge du cache dev n'est autorisée.

Les manifestes sont créés sans écrasement, distinguent snapshot courant et
révision d'origine d'une image réutilisée, et conservent les IDs vérifiés.
Les images créées sont enregistrées dans le registre Docker append-only du
projet ; ni images ni caches ne sont nettoyés automatiquement. Les contextes
temporaires possédés sont retirés en fin de commande.

## Limites de qualification

Les oracles hors ligne ne prouvent ni une compilation Linux, ni un push/pull OCI,
ni le bootstrap/trust Zot/Kind, ni un rollout, ni le scénario OpenBao/OAuth.
Publication OCI, raccord registre et refresh applicatif sans reprovisioning
restent des étapes distinctes ; ne pas utiliser un ID Docker comme digest OCI.

Référence du contrôle de builder : [Docker buildx inspect](https://docs.docker.com/reference/cli/docker/buildx/inspect/).
