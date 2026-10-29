# Comparaison temporaire Ubuntu22.04 — contrat d'auteur

## Entrées et admission

Lire le ledger `.cursor/handoffs/20261029-k6-01-socle.md`, rapport23 de la mission `team_mvtz6p47_000q_4wmht0` et le nouvel addendum du contrat `docs/contracts/k6-functional-linux.md`. Le lead fournira son commit de ratification avant admission de l'auteur. ADR0504/V1 restent Accepted/Partial ; Ubuntu24.04 reste la qualification nominale. Seule l'expérience temporaire22.04 est admise, pas la recommandation future de profil AppArmor.

Faits : run40973768642/sourcecf9f5ca, 96tests Linux OK sans skip et outils PASS ; k6 refuse avant le cas ; le même Chromium direct signale NO_USABLE_SANDBOX_REPORTED. Restriction AppArmor1 observée, pas d'errno corrélé ou cause exclusive. La comparaison22.04 teste une dépendance à l'environnement, pas AppArmor seul. Runner22.04 en dépréciation avec retrait2027-04-17 : aucune migration durable ou qualification24.04 implicite.

## Ownership et modifications admises

Même propriétaire outillage que les derniers lots, worktree existant préservé. Écrire seulement `ops/tests/k6/**` et `.github/workflows/k6-functional-smoke.yml`. Ne pas recopier les gardes ou créer un second harness. Ne pas toucher lock/pins/scénarios, service/identité/SDK/CI existante, ADR ou contrat. Pas reset/rebase de l'ancien index ; rendre le nouveau delta par rapport au HEAD intégré fourni, pas aux cinq fichiers historiques déjà capturés.

- Une entrée diagnostique fermée, explicitement distincte des commandes de qualification, pour Ubuntu22.04 amd64 non-root. Les commandes nominales continuent à refuser22.04 ; aucun sélecteur d'OS libre, faux os-release, fallback, URL/args/exécutable libre ou contournement du lock24.04.
- Un seul cas k6/browser, mêmes pins/ELF/SHA, environnement, arguments et scénario about:blank/évaluation/750ms existants. Outils/versions et deux inspect avant lancement ; réutiliser les helpers et Child. Un direct22 n'est pas nécessaire à cette question et n'est pas demandé. Aucun retry, paquet, helper, sudo ou changement de sécurité.
- Même si le cas complet, les checks, témoin sandbox, confidentialité et cleanup réussissent, HTTP/Chromium restent INCONCLUSIVE et la comparaison sort non-zéro. Rapporter séparément les faits de démarrage/cas/témoin, sans PASS de plateforme ou résultat synthétique. Pas d'appel publish pour cette expérience ; seul JSON fermé et borné admis. Les échecs privacy/cleanup restent bloquants, pas neutralisés par une observation utile.
- Relever l'environnement effectivement contrôlé et les métadonnées déjà admises ; labels de comparaison distincts et explicitement non équivalents à24.04. Les valeurs libres ne sont jamais recopiées sans validation fermée ou format restrictif existant. Aucun nouveau scan/profil/journal/sonde privilégiée.
- Workflow dédié : conserver le job24.04 et ses résultats sans masquer son échec ; ajouter seulement le chemin diagnostique22.04 nécessaire. Un push normal peut relancer incidemment le job canonique, pas de dispatch/retry manuel ni revendication A/B à une variable. Actions SHA, permissions contents:read, checkout sans credentials, acquisition explicite, racine privée et absence d'upload d'échec conservées. Aucun secret/déploiement/tunnel ajouté.

## Tests et preuves

ROUGE avant correction lorsque possible, puis GREEN Windows natif avec temp hôte approuvé : nominal22.04 refusé avant Child, comparaison22.04 admise,24.04/OS inconnu/root/architecture inconnue refusés par cette entrée ; pins/identités/versions/inspect avant browser ; résultat positif simulé toujours INCONCLUSIVE/non-zéro/non publié ; scénarios négatifs et sorties forgées/sentinelles ; privacy/cleanup/témoin inchangés. Les fixtures peuvent simuler des résultats locaux mais ne doivent jamais se présenter comme un smoke réel. Ne pas répéter un run Linux ou lancer Chromium dans le worktree.

Réutiliser les sources officielles et recherches du rapport23 ; recherche additionnelle ciblée seulement pour un choix nouveau. Pas de Cargo/hook dans le long worktree, aucun runtime/CI/push/installation par l'auteur. Lead : intégration, hook normal, replay ciblé, publication puis essai Linux. Rendre fichiers et diff exact, commandes effectivement exécutées, ROUGE/GREEN/limites, état staged/unstaged, aucune ressource runtime créée. Geler le candidat à la livraison.
