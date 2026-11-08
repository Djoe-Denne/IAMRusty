# ADR-0608 : Recréer le seul lab local-full pour activer OAuthConnect, sans migration de ses données

- Statut : Proposed
- Réalité : Unimplemented — reconstruction et activation non qualifiées à la rédaction
- Date : 2026-11-08
- Décideurs : autorisation opérationnelle explicite de l'utilisateur le 2026-11-08 ; Accept du texte final distinct
- Jalon concerné : Local-full / Vague 4 locale, hors Apparatus P0–P6
- SuperSède : aucune
- SuperSédée par : —
- Related : [0606](0606-local-full-kind-isole.md), [0607](0607-images-source-snapshot-registre-zot-hors-kind.md), [0413](0413-oauthconnect-multi-instances-profils.md)
- Contrat d'exécution : [local-full-oauthconnect-recreation.md](../contracts/local-full-oauthconnect-recreation.md)

## Contexte

L'utilisateur demande un ADR avec limitations puis l'activation rapide, et autorise la destruction/recréation de son environnement local plutôt qu'un chantier de convergence en place. Cette permission est bornée au cluster **`aiforall-local-full`**, contexte `kind-aiforall-local-full`, trois nœuds du bail attesté ; elle ne concerne pas le legacy ni les autres ressources locales.

Le [ledger de reprise](../../.cursor/handoffs/20261108-ops-oauthconnect-reprise.md), §§ GO activation locale / Réorientation explicite utilisateur, rapporte : neuf images applicatives et artefact prêts ; premier déploiement refusé avant mutation pour binding d'ancre, corrigé ; deuxième apply foundation partiel possible puis échec sur quatre conflits SSA, restauration des replicas des vingt Deployments à leur baseline rapportée. OAuthConnect n'était pas installé lors du dernier constat. Ces faits du parent ne sont pas une nouvelle inspection live par l'architecte.

## Décision

**Remplacer uniquement ce lab par une installation neuve OAuthConnect, avec perte acceptée de ses DB/PVC et du coffre local, au lieu de réconcilier son installation existante.** Ce choix de stockage destructif et de frontière de reconstruction justifie une ADR distincte ; il ne ratifie ni ne SuperSède 0606/0607.

1. Inventorier et attester les trois IDs de nœuds, volumes/montages et baux avant destruction. Retirer seulement les ressources dont l'appartenance à ce full est établie. Identité/propriété ambiguë ou bail actif incompatible ⇒ STOP et arbitrage ciblé, pas adoption par nom.
2. Préserver le cluster legacy et tous les autres clusters, PostgreSQL opaque et volumes inconnus, registre OCI externe, images/caches/builder Docker, credentials GitHub/simulés et dossiers PKI privés. Aucun prune global ni `force-conflicts`.
3. Recréer un control-plane et deux workers, avec **nouveau bail v2 et kubeconfig privé**, sans réemployer l'identité API/UID de l'ancien cluster. L'ancrage endpoint/mapping précède tout GET API ; l'UID est vérifié avant mutation.
4. Réemployer les **neuf images déjà construites et leur artefact**, après vérification de leur provenance courante ; recharger explicitement les nouveaux nœuds et émettre le reçu adapté. Le parcours autorisé est le `--build-manifest` / `kind load` existant : exception locale explicite au nominal registre proposé par 0607, pas abandon de ses contrôles de fraîcheur ni réparation du registre externe.
5. Déployer le profil existant **`--oauth-connect`**, instances `github` et `simulated`, depuis la **PKI complète courante** et les inputs privés conservés. Pas de rotation de CA/clés ni de régénération générale ; ne pas reconduire le vieux dossier incomplet ou les références historiques incohérentes. Première installation : **sans `--preserve-platform-tls` ni mode réseau de compatibilité de l'ancien environnement**. TLS/mTLS et vérifications CA/SAN/hostname/validité restent obligatoires.
6. Utiliser une nouvelle registration lifecycle liée au nouveau bail, avec les deux sources privées. Le keepalive explicitement demandé s'applique au nouveau lab ; le résultat d'activation reste à prouver, pas à déduire de cette permission.

## Limitations et dette conservées

- Le déploiement global actuel mêle installation et migration ; le succès sur un lab neuf ne qualifie pas un upgrade ou un refresh préservant les données. Les dérives manuelles NetworkPolicy et sources de certificats de l'ancien lab ne sont pas résolues comme procédure de convergence.
- La reprise après expiration pendant un arrêt Docker reste **non prouvée** ; aucun certificat inspecté dans cette session n'était expiré. La [note TLS](../guides/local-tls-reprise.md) décrit le parcours antérieur de conservation, non le mode à employer pour cette reconstruction. Aucune dérogation TLS n'en découle.
- Ce choix local n'est **ni une procédure production, ni une migration avec préservation des données, ni une preuve HA/cloud ou compatibilité GitLab**. `simulated` demeure un fournisseur simulé distinct. Pas de qualification OAuth navigateur sur la seule base de Pods Ready.
- Les correctifs utiles déjà testés restent récupérables ; le parent documente leur disposition explicite. Ni retrait global des corrections ni poursuite du chantier de réconciliation (`recover_apply_receipts.py`, nouveau mode réseau) exigés pour cette activation.

## Alternatives écartées et hors décision

- Poursuivre maintenant la convergence de l'ancien lab : contraire à la priorité utilisateur ; dette conservée explicitement, pas déclarée résolue.
- Forcer les conflits SSA, désactiver TLS/réseau ou supprimer des ressources étrangères : hors autorisation.
- Reconstruire cloud/prod, migrer le legacy ou créer un nouveau pipeline : hors périmètre. Accept des ADR Proposed inchangé ; aucun Accept autonome.

## Sources et preuves

- Canon : ADR0606 (isolation, bail v2, stockage local) et ADR0607 (provenance/cache, distribution proposée).
- [Procédure Ops existante](../../ops/deploy/README.md#oauthconnect--github--fournisseur-simulé-entrée-https) : profil, inputs privés, chargement explicite, TLS et distinction premier déploiement/reprise.
- [Ledger de mission](../../.cursor/handoffs/20261108-ops-oauthconnect-reprise.md), fin du 2026-11-08 : autorisation et état des tentatives. Il porte les preuves runtime futures.
- Cette rédaction est documentaire seulement : aucune exécution runtime ni nouveau test ; **aucun PASS d'activation revendiqué**.
