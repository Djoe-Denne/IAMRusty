# 4. Portage en cours, cible réelle manquante

## Objectif et état

Livrer une campagne authentification k6 Windows avec les mêmes garanties métier. La matrice complète est intégrée et le contrat minimal ratifié. Les scénarios et leur raccord Windows sont maintenant en cours chez deux auteurs isolés ; aucune requalification générale du framework n'est lancée.

La décision demandée porte uniquement sur la cible de la future exécution : les entrées attendues n'écoutent pas, et le laboratoire historique n'est plus identifiable sur le daemon Docker courant. Cela n'empêche pas le code autorisé de progresser, mais empêche une preuve réelle de parité.

## Changements et résultats vérifiés

- Le dernier navigateur qualifié utilisait un compte déjà inscrit. La matrice conserve cette limite : le nouveau scénario doit réaliser une inscription puis une nouvelle connexion, avec des compteurs adaptés et vérifiés avant run.
- Matrice et contrat sont committés dans master. Le hook existant de commit a exécuté cargo fmt --all -- --check avec succès ; aucun test Rust n'a été rejoué et le slot Cargo est rendu.
- Recherche Windows : zéro listener18080/18081. Docker Desktop et son backend étaient déjà actifs avant inspection ; aucun réveil. Sur le contexte desktop-linux, la sélection par label exact du laboratoire local-full ne retourne aucun conteneur. L'ID du control-plane historique est explicitement absent. Ce n'est donc pas un simple conteneur historique arrêté que nous pouvons reprendre tel quel.
- Aucun appel applicatif, lancement k6/Chrome, création de ressource, déploiement, tunnel, restart ou suppression. Docker/WSL préexistants restent inchangés et ne sont pas possédés par cette mission.

## Options et recommandation

**Fournir une cible existante** : transmettre sa référence et son bail non secrets pour vérification, avec les origines/autorités historiques conservées et joignables depuis Windows. Nous pourrons préparer puis exécuter les scénarios après intégration et revue si la cible est effectivement admise. Aucune permission de créer un laboratoire n'en découle.

**Différer l'exécution réelle** : continuer à intégrer les scénarios et le raccord minimal, puis documenter précisément le prérequis manquant. Aucun PASS d'authentification k6 ni clôture de parité ne sera annoncé. Cela ne change ni les garanties ni les preuves historiques.

Je recommande de fournir une cible existante si vous en avez une ; sinon, différer le run tout en poursuivant le portage. Créer ou redéployer un laboratoire dépasserait le contrat approuvé et n'est pas proposé comme action implicite. Aucun délai ou coût total n'est estimé.

## Décision demandée

Pouvez-vous fournir une cible existante autorisée et son bail non secret pour la campagne Windows, ou souhaitez-vous différer la preuve réelle en conservant le travail de portage en cours ? Ne transmettre aucune clé, JWT, cookie ou autre secret en chat.

## Annexe de preuves

Commit local `dde0340915905953c861ad0aa1c164a111f20db1`, publication distante pas encore effectuée. Documents : docs/contracts/k6-windows-auth-functional.md (Accepted/non implémenté) et docs/missions/20261031-k6-auth-parity.md. Candidat recherche result_mvwjcrh0_000b_hq8ghb et relecture lead du ledger3742–3764. Auteur JS pool-3/tâche task_mvwjnknv_000g_6tiu8a ; auteur parent parent-auth-windows/tâche task_mvwjnknv_000h_ndajal.

Commandes observatrices : Get-NetTCPConnection18080/18081 ; Get-Process DockerDesktop/backend/vmmemWSL ; docker context show ; docker container ls -a --no-trunc --filter label=io.x-k8s.kind.cluster=aiforall-local-full ; docker container inspect --format Id/State sur6fd561584da07159621e52e04580dacbbc0dfabdf7a79a221e730ea2da89b7f1 → No such container. Premier format incluant label échoué sur guillemets PowerShell, corrigé sans nouvelle opération : pas échec produit ni panne du runtime. L'absence est vérifiée pour cette cible/ce daemon, pas pour tous les environnements possibles.
