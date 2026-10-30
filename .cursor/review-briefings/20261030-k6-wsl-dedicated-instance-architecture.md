# Hôte WSL de projet dédié sur I: — arbitrage avant création

## Demande explicite et périmètre

L'utilisateur remplace l'installation dans ARTIST-Ubuntu par une nouvelle instance
WSL sur I:, nom exact **AI for All**, et autorise toutes les installations utiles
dans cette instance dédiée au projet. Ne pas redemander une autorisation pour les
quatre bibliothèques ou les outils nécessaires à cet hôte neuf. Cette autorisation
ne s'étend pas à Docker, aux autres distros, à un changement Windows global,
au no-sandbox, aux secrets ou à la publication de données natives inconnues.

Mission1/ADR0504/V1 Accepted/Partial inchangés, cible Ubuntu24.04 nominale et user
non-root pour les tests. Builds/tests Rust de dev toujours Windows natif, slot
Cargo unique ; cet hôte sert aux outils k6/Chromium Linux et au diagnostic privé
éventuel. Pas de nouveau déployeur/tunnel/service ou autre mission.

## Faits vérifiés par le lead

- ARTIST-Ubuntu24.04 était revenu Stopped après préflight RO, aucun paquet installé.
  docker-desktop Running/étranger, default WSL préexistant docker-desktop à préserver.
- I: NTFS/Healthy, taille1248762785792octets, libre1240483057664octets ; I:\WSL existe,
  **I:\WSL\AIForAll absent**, aucun dossier ou distro créé pour la demande nouvelle.
- WSL2.6.1.0, noyau6.6.87.2-1 ; l'aide locale expose --install --name --location
  --no-launch et --import. --help retourne255 mais fournit bien cette aide ; pas
  d'échec de création déduit de ce code d'usage. Nom avec espaces à conserver si
  accepté, ne pas essayer d'autres noms implicitement sur un refus réel.
- Documentation Microsoft raw basic-commands.md relue (location/no-launch/import),
  guide Ubuntu WSL officiel liste Ubuntu-24.04 distinct de Ubuntu/26.04. Le lead
  choisit24.04 pour rester conforme ; acquisition officielle/identité/provenance
  à tracer, aucune mise à jour de WSL ou fonctionnalité Windows nécessaire présumée.
- Source k6 exécutée1c0036f,140Linux OK22/24 ; nouveau WSL non encore qualifié.
  Private-site channel Proposed, pas de relevé ou consommateur actuel.

## Livrable documentaire seulement

Lire la version canonique actuelle du nouveau contrat causal et du ledger, pas
la copie stale de votre ancien worktree. Ajouter dans **le nouveau contrat causal
seulement** un petit addendum Proposed de rattachement à cet hôte dédié :
provisionnement/acquisition Ubuntu24, nom/emplacement exacts, nouveau UID utilisateur
non-root, administration possible uniquement pour l'installation de l'hôte neuf,
identité/registre/bail/états avant-après, données privées sur son FS Linux natif,
persistance de la distro/données demandée mais runtime à la demande et stop possédé.

Séparer ce qui peut être ratifié avant création (provisionnement dédié demandé)
de la future admission du relevé source privé (canal/QA requis). Pas de permission
globale inventée ou de changement de pin/args/protection. Aucun compte existant,
ancienne distro ou dossier occupé remplacé ; pas de changement de distro par défaut,
reset/unregister/prune. Ne pas installer spontanément une toolchain Rust ou Docker.

Réutiliser les sources et choisir les détails locaux raisonnables : il ne s'agit
pas d'une nouvelle mission ou de quatre nouvelles ADR. Le lead ratifiera avant
création si la proposition respecte le mandat. Aucun code, runtime, paquet,
CLI WSL, Cargo/hook, CI ou ledger partagé par l'architecte ; geler le fichier et
rendre le delta exact, sources et diff-check réel. Les préflights Windows en
lecture et cette décision peuvent avancer indépendamment, pas les mutations WSL.
