---
title: "Authentification — qualification locale du 29 octobre 2026"
category: references
tags: [security, architecture, visibility/internal]
sources:
  - .opencode/missions/team_mvtz6p47_000q_4wmht0/reports/99-cas-minimal-windows-verifie.md
  - docs/reviews/20261029-authz-proposals-carried-forward.md
  - docs/reviews/20261029-worktree-consolidation.md
  - docs/reviews/20261029-auth-security-final-evidence.md
  - docs/missions/20261029-auth-security-publication.md
  - docs/contracts/20261029-auth-security-qualified-state.md
  - .cursor/handoffs/20261028-auth-security-closure.md
  - docs/missions/20261029-k6-recherche-et-enchainement.md
summary: "Qualification locale OAuth et Envoy, correctif d'autorité et limites conservées. Le cas minimal k6 Windows ultérieur ne vaut pas portage de cette preuve OAuth."
provenance:
  extracted: 1.0
  inferred: 0.0
  ambiguous: 0.0
created: 2026-10-29
updated: 2026-10-31
---

# Authentification : ce qui est qualifié, et ce qui ne l'est pas

## Résultat intégré

[[projects/iamrusty/decisions/0412-oauth-transaction-browser-binding]] est Accepted/Implemented uniquement pour le laboratoire ratifié. Le correctif impose l'autorité issuer/audience aux deux constructions production de TokenService, sans fallback legacy. L'image IAM corrigée et le navigateur ont été qualifiés sur le commit applicatif `7b8c1f95` ; les commits suivants de QA/documentation n'ont pas changé ces sources.

Le vrai navigateur Chrome réussit départ login, connexion 200, départ authentifié et callback exact sans Bearer ; trois scénarios négatifs retournent 403. Le connecteur configuré utilise un fournisseur synthétique. Un client HTTP seul n'aurait pas constitué cette preuve navigateur.

Envoy refuse réellement un certificat portant le mauvais nom malgré la bonne autorité de certification : bon nom 303, mauvais nom 503 avec hausse du compteur SAN, bon nom restauré 303. Un simple rendu de configuration ou un test reqwest ne prouve pas ce comportement.

## Conclusions utiles des spécialistes

Le contrat qualifié consolide les restitutions indépendantes d'architecture, de sécurité, d'oracles de tests et d'infrastructure, avec leurs références de briefings ; ce texte n'est pas un export exhaustif des conversations des agents.

- Architecture : réconciliation locale de 0412 proposée puis explicitement ratifiée ; pas de promotion transitive des ADR JWT, mesh ou signataire.
- Sécurité et tests : aucun défaut élevé ou critique connu encore ouvert dans le périmètre revu. Les corrélations rendent les refus recevables, mais le refus avant IAM et l'absence de mutation restent inférés, sans mesure SQL directe.
- Infrastructure : qualification bornée du laboratoire, pas validation globale production/HA. Les limites de compteurs manquants et d'absence de config_dump complet restent explicites.
- Trajectoire k6 : les trois étapes initiales de [[projects/aiforall/concepts/k6-functional-linux]] restent distinctes. Au 31 octobre, seul le cas minimal Windows about:blank/42 est PASS ; ni la preuve OAuth décrite ici, ni les campagnes de droits ne sont portées ou qualifiées par ce résultat.

## Preuves et ressources

Les suites finales Windows totalisent 17 PASS, 0 échec, 0 ignoré et 0 filtré : transactions navigateur 9, PostgreSQL 6, cleanup 2. Les gates purs rapportent 19 tests domaine, 3 tests RS256, check IAM et fmt réussis. Les preuves historiques principal/signataire ne sont pas recomptées comme exécutions du correctif OAuth final.

Trois fixtures PostgreSQL créées puis retirées. Chrome et forwards fermés ; trois nœuds Kind possédés arrêtés gracieusement, sans suppression. Les conteneurs étrangers sont préservés ; Docker Desktop/WSL ne sont pas arrêtés globalement.

## Limites et dettes

Production, haute disponibilité, deux répliques live et projections runtime exhaustives ne sont pas acquises. La suite Manifesto complète n'est pas PASS après erreurs d'environnement LNK1102/E0463 ; l'enfant SDK Windows réel n'a pas été exécuté. Le SDK utile est publié sur sa branche de mission, pas fusionné automatiquement dans la branche principale distante.

La qualification matérielle HSM est une dette acceptée non bloquante pour les futures campagnes k6, pas une capacité réalisée. ADR 0309 conserve Accepted/Partial. [[projects/aiforall/decisions/0308-mesh-authn-jwt]] conserve également son statut propre.

## Provenance et exclusions

La consolidation complémentaire fait l'objet d'une revue statique indépendante
dans `docs/reviews/20261029-worktree-consolidation.md` : pas de perte fonctionnelle
bloquante établie en conservant les versions finales. Deux précisions de l'ancien
contrat Proposed (liaison store FGA/autorité et invitations UUID account-only)
sont reprises dans `docs/reviews/20261029-authz-proposals-carried-forward.md`.
Architecte en est l'auteur ; le lead ratifie leur **conservation documentaire
uniquement**. Elles ne deviennent ni exigences Accepted ni nouveaux tests exécutés.

Après ces dispositions, les rapprochements natifs ont conservé la version finale
et la mission a été archivée :33tâches terminées, aucun worktree de ce lot restant
dans Git ou OpenCode. Les copies anciennes ont été sauvegardées avant retrait,
sans les promouvoir ; les permissions de réparation temporaires ont été retirées.
Les autres anciens arbres et les fichiers étrangers du checkout restent préservés.

Sur demande explicite, les ajouts d'autorisation d'édition des24profils de projet
ont également été commités. Cette capacité native n'annule pas les mandats de
revue READONLY. Les contrôles fmt finaux de l'application et du SDK sont verts ;
ils ne constituent pas une nouvelle campagne fonctionnelle.

Les conclusions de sécurité et d'infrastructure sont maintenant versionnées dans
`docs/reviews/20261029-auth-security-final-evidence.md`, sans les sorties brutes.
Le document `docs/missions/20261029-auth-security-publication.md` trace la
publication et les arbitrages d'intégration : les livrables des worktrees sont
à intégrer, pas leurs dossiers techniques ; l'alternative ADR0315 a été écartée
explicitement au profit du contrat remote ratifié. Les versions Proposed restées
dans ces arbres ne remplacent pas les documents ratifiés de master.

Sources canoniques : contrat qualifié et ledger de clôture cités en frontmatter. Ils référencent les revues et preuves détaillées. Aucun JWT, cookie, code OAuth, clé, profil navigateur, journal brut privé ni raisonnement interne d'agent n'est ingéré. Les futures missions sont documentées dans le dossier `docs/missions/20261029-k6-*`, pas admises par cette ingestion.

Retour au projet : [[projects/aiforall/aiforall]].
