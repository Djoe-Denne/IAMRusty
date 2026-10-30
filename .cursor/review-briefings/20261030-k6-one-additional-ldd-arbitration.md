# Arbitrer une seule mesure ldd après défaut d'instrumentation démontré

## Mandat et sources

Même architecte12/session/profil/modèle. Consultation documentaire distincte de
la proposition Windows après exit en cours ; conserver celle-ci comme livrable,
sans code applicatif ou runtime. Lire les contrats Accepted
`k6-chromium-linux-probes.md` et V1, rapports71/72, et
`20261030-k6-ldd-loader-parser-fix.md`. Canon source durable f279, docs de résultat
plus récentes non committées : lire les chemins canoniques absolus.

L'utilisateur a repris pour utiliser ses recherches namespaces/dépendances puis,
si cela ne débloque pas, envisager Linux natif ou éventuellement k6Windows. La
délégation long run admet les décisions nécessaires dans mission1, pas un GO
humain à chaque jalon. Aucune migration/hôte nouveau encore choisi ou disponible.

## Faits actuels

- Une seule commande userns, succès réel dans Child. Aucune seconde demandée.
- Une seule commande ldd, exit0 mais compte inconnu ; captures effacées par
  contrat, impossible de requalifier rétrospectivement sa sortie.
- Défaut statique du parseur démontrable : une ligne chargeur reconnue ne couvre
  pas son basename dans names, alors que elf_metadata conserve ce DT_NEEDED.
- Lecture fichier ELF seule, déjà admise :32DT_NEEDED, chargeur direct=TRUE,
  archive/ELF/chargeur/4sources revérifiés. Aucun nouvel outil/commande externe,
  nouveau unshare/ldd ou navigateur exécuté pour ce fait.
- Correction minimale du parseur et fixtures demandées à auteur13 ; aucun mode
  consommateur/runtime autorisé. Sources OPS et pins gelés.
- Privacy/cleanup/retrait Linux et copies parent vérifiés ; handles Windows
  conservés/joints avec identité avant et création/fin par même handle ; runtime
  finalementStopped. Pas de collecte/FATAL privé. Cause Chrome toujours inconnue.

## Décision demandée avant implémentation consommatrice

Le contrat ponctuel dit « chaque commande au plus une fois par voie, sans
retries ». Il n'est pas contourné en renommant la voie ou en changeant d'hôte.
Examiner l'exception minimale : **UNE invocation ldd supplémentaire seulement**,
après correctif démontré ROUGE/GREEN et revue lead, mêmes pins/UID/ELF/env/
superviseur/timeout/privacy/identités/cleanup, sous bail neuf. Zéro unshare,
Chrome/k6/--version, réseau, installation, système, chargeur lancé manuellement,
LD_DEBUG/logging ou collecte brute supplémentaire. Pas de fallback automatique.

Le but n'est pas un PASS Chrome mais une mesure exploitable du nombre de
bibliothèques directement/transitionnellement rapportées absentes par ldd.
Même compte0 ne prouve pas les bibliothèques dlopen tardives ni la cause Chrome.
Si encore inconclusif, conserver la cause/limite précise et instruire le repli
Linux natif plutôt qu'une boucle de reconnaisseurs/reruns.

Une sélection **temporaire explicite ldd-only** paraît le plus petit raccord,
sans modifier ops/tests/k6 ni créer un autre superviseur. Préciser le schéma et
le code de sortie honnêtes : unshareNOT_EXECUTED, preuve de succès userns ancien
réutilisée séparément ; aucune ligne fictiveZERO_EXIT ou PASScollé du run ancien.
Ne pas élargir l'instrumentation ou le format si les sorties fermées existantes
suffisent. Les Negative tests, signal/overflow, cleanup et retraite restent.

Décider compatible ou besoin d'autorité réellement nouvelle, avec justification
ancrée. Si compatible, écrire seulement un nouveau
`docs/contracts/k6-ldd-parser-completion.md` Proposed, bornant l'exception et son
compteur global ; ne pas modifier Accepted/ADR/index/ledger/OPS. Le lead lira et
ratifiera avant code/runtime s'il reste dans le mandat. Pas de nouvelle ADR pour
un détail diagnostique réversible. Aucun script, fixture, commande Windows/WSL,
Cargo/CI, navigateur, installation, ACL ou sous-agent par architecte.
