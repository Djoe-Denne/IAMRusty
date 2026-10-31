# 1. Préparation de la campagne authentification Windows

## Objectif et stade

Obtenir sous Windows une campagne k6 conservant les garanties d'authentification déjà qualifiées. Préparation seulement : aucune exécution approuvée. Recommandation : garder la QA proportionnée, avec matrice de parité, contrat adapté avant code et revue indépendante ciblée.

## Résultats vérifiés et limites

Le socle Windows et ses archives sont ancêtres du checkout actuel. Ses documents rapportent34tests Windows sans omission et le navigateur minimal réussi ; preuves historiques non rejouées ici, qui ne qualifient ni HTTP ni OAuth.

Le lanceur refuse encore HTTP sans cible admise. La campagne historique avait arrêté son laboratoire : aucune cible actuellement disponible n'est établie. Ni nouveau laboratoire, déploiement, tunnel ou réveil implicite Kind/WSL ne sont prévus. Sans cible admise, le code peut avancer dans son contrat mais la qualification réelle reste non exécutée.

Le pilote source contient inscription puis nouveau login, réassociation réelle et refus. La traduction doit préserver aussi les observations parent et les cookies/redirections, pas seulement les requêtes. Les contrats Windows ne couvrent aujourd'hui que le socle.

## Aperçu proposé après start

1. Inventorier toutes les garanties et leurs preuves conservées.
2. Faire adapter puis ratifier le contrat Windows et l'admission de cible par l'architecte.
3. Porter les scénarios JavaScript et le parent Windows en parallèle sur fichiers distincts.
4. Tester le contenu intégré ; relire indépendamment les assertions et frontières de sécurité modifiées.
5. Qualifier seulement sur cible existante admise, restaurer/fermer les ressources possédées, puis intégrer et publier Windows.

Voir le [plan détaillé et ses dépendances](../prepared-plan.md). Aucun worker ni tâche native n'a démarré ; aucun classifieur ou score de difficulté requis.

## Politique réelle proposée

Version2, catalogue fourni inchangé, seuil de planification50, classification désactivée, sans délégation long_run. Chaque tâche garde session/modèle ; variantes de raisonnement dans le catalogue approuvé. Les métriques ne garantissent ni durée ni coût total.

Catalogue conservé (intelligence/coût indicatif par tâche, unité du catalogue) :
- Claude Haiku5.5 : low29/0.02, medium34/0.05, high38/0.08, xhigh41/0.12, max43/0.21.
- DeepSeek4.1 flash : max39/0.27.
- GLM5.3 flash via zai et opencode-go : max42/0.25 chacun.
- GPT6.1 Sol : low42/0.13, medium48/0.21, high50/0.32, xhigh51/0.39.
- Gemini3.8 flash : medium40/0.93, high41/1.24.

QA **proportionate**, commandes existantes exactes et critères ajoutés à [verification dans la politique](../model-policy.json). Les futures commandes auth seront fixées avant run, pas inventées aujourd'hui comme disponibles. Les17tests d'intégration restent Windows et sont réutilisés/rejoués selon comparaison des sources. Pas de nouvelle infrastructure de preuve.

## Options et décision demandée

Garder **proportionate** apporte les contrôles liés aux risques réellement changés et conserve les revues obligatoires. **Light** réduit seulement les contrôles optionnels, sans supprimer parité/revue imposées. **Reinforced** ajoute un plan explicite et revue bornée, sans autorisation de laboratoire. Garder la pause reporte le portage sans changer le produit. Aucune estimation fiable de durée/coût total n'est disponible.

Vous pouvez corriger en chat le périmètre ou ce choix QA. Puis afficher la révision exacte avec `/ensemble-mission config` et, seulement si elle convient, lancer `/ensemble-mission start`. Le lead n'invoque pas start et un accord en chat ne le remplace pas.

## Annexe de preuves

- HEAD `05941d163eb6632f26c89fc7057af03880148a08`, master, SDK02e5 ; un worktree principal. Commits201ec05b/a24a7ce5 ancêtres, exits0. Branches historiques et changements étrangers préservés.
- Sources lues : mission1/mission2, ADR0504, contrats k6 Linux/Windows/version, état qualifié auth, contrats0412 exact-route/alignement/autorité, pilote historique et entrée Windows.
- Localisation : futurs scénarios auth et raccord Windows existant. Impact : outillage/docs, pas produit Rust. Exploration symbolique Rust non applicable à cette préparation ; workflow MCP requis si elle devient nécessaire.
- Exécuté : lectures, inventaire Git et sous-module, diffstat, ascendance. Aucun test, k6/Chrome, HTTP ou runtime ; aucune ressource Docker créée.
- Premier batch Context Mode mal formulé : commandes PowerShell envoyées au shell POSIX, non exécutées. Lectures corrigées via JS/fs dans Context Mode ; aucune panne MCP. Aucun résultat de test déduit de ces lectures.
