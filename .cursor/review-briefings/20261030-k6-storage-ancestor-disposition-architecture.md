# Disposition des ancêtres Windows observés — investigation documentaire parallèle

Même architecte12, pas de code/runtime. Mandat : instance dédiée AIForAll sur I:
demandée par l'utilisateur, installations nécessaires autorisées, disque/outils/
données à préserver ; pas de mutation étrangère, arrêt global, unregister/reset,
nouveau déployeur ou changement de sandbox. Hôte/lecteur déjà choisis.

Lire le canon ACTUEL principal, rapport62, revue `20261030T1428Z` et ledger.
S-1 du conducteur concerne les ancêtres Windows avant Process.Start ; auteur13
le corrige avec S-2 rétention dans les trois fichiers du conducteur. Ce travail
indépendant ne doit pas bloquer ou adapter ses gardes pour faire passer l'hôte.

## Observations du lead, RO, sans WSL

Get-Item/Get-Acl sur I:\, I:\WSL, I:\WSL\AIForAll et ext4.vhdx : aucun reparse
observé, feuilles privées/protégées courant/SYSTEM/admins. I:\WSL hérité non
protégé, propriétaire courant, ACE Allow AuthenticatedUsers applicable à l'objet :
DELETE et création fichier/dossier, pas DELETE_CHILD sur cette ACE. I:\ porte
une ACE analogue ; propriétaire non account-SID résolu et hors token courant,
identité exacte non publiée et rôle privilégié éventuel non encore caractérisé.
Deux diagnostics exit0, aucun exploit ou accès effectif complet prouvé. Ne pas
conclure une compromission ; ne pas déclarer la chaîne sûre à partir des feuilles.

Sources officielles recoupées :
https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights
https://learn.microsoft.com/en-us/windows/win32/fileio/file-access-rights-constants

## Question technique à résoudre dans le mandat

Proposer la disposition minimale pour que le parent puisse ultérieurement vérifier
une chaîne privée sûre, **sans modifier automatiquement I:\ ou I:\WSL partagés**.
Étudier une seule alternative concrète si utile : nouvelle chaîne de premier
niveau privée sur I: et déplacement natif de la seule instance possédée, conservant
GUID/nom/données, si la version et les préconditions le permettent. Pas de clone/
réimport/unregister, adoption d'une autre distro, changement du défaut ou suppression.
Une possibilité n'est pas une autorisation d'exécuter ni une preuve de sûreté.

Vérifier docs officielles WSL/locales déjà connues pour `--manage --move`, sa
sémantique/limites, ce qui doit être observé sans runtime, la sauvegarde récupérable,
l'inventaire et la propriété, le traitement de l'ancien chemin, et le cleanup.
Préciser les autorités réellement déjà acquises et tout arbitrage indispensable,
sans catégorie d'approbation automatique ou demande de GO générique. Ne pas ajouter
de politique, service, daemon ou harness pour compenser un chemin douteux.

Livrer recommandation bornée, limites et plus petites observations restantes ;
petite précision **Proposed** au seul contrat causal si une frontière ouverte le
justifie, en lisant sa version actuelle avant écriture. Sinon retour READONLY.
Pas d'édition du conducteur, ledger partagé ou autre contrat, aucune exécution
WSL/Windows mutation/Cargo/CI, nouvel utilisateur, changement ACL ou déplacement.
Le lead intègre et arbitre avant toute opération ; aucune collecte réelle maintenant.
