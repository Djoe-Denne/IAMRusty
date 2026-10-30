# Instance WSL dédiée AIForAll

## État courant — approche suspendue, qualification inconclusive

Le2026-10-30, l'utilisateur demande d'abandonner cette approche pour le moment,
d'archiver les outils créés pourWSL et de poursuivre les essais sousWindows.
**Chrome/WSL reste INCONCLUSIVE**, pas PASS : le dernier navigateur échouait
avant le cas et sa cause n'a pas été établie. La réussite du testuserns et les
tests unitaires ne changent pas ce verdict. Le premierldd reste compteinconnu ;
la seconde mesure envisagée est annulée avant exécution.

Cette suspension ne supprime pas l'instance, sonVHDX surI:, ses données ou ses
outils installés. Aucun nouveau lancementWSL ou collecte n'est admis pour la
poursuite de cette enquête. Les scripts ponctuels utiles sont destinés aux
archives documentaires, pas à une relance automatique.

Les résultatsWindows seront qualifiés séparément. L'utilisateur prévoit une
missionLinux ultérieure sur une machineGitHubActions ; elle n'est pas lancéeici.
[Décision et limites](../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/76-arret-wsl-et-preparation-des-tests-windows.md).

## Identité vérifiée

- Nom technique : `AIForAll` ; libellé du projet : AI for All.
- Stockage : `I:\WSL\AIForAll\ext4.vhdx`, WSL2, Ubuntu24.04.
- Compte de travail et compte par défaut de cette instance : `aiforall`, UID1000,
  HOME `/home/aiforall`, permissions0700 sur le FS Linux natif ext4.
- GUID observé à la création : `{22ce910b-45a3-42a4-bbab-dc56a1766035}`.
- L'ancienne distribution ARTIST-Ubuntu et le défaut WSL existant n'ont pas changé.

Création explicitement demandée par l'utilisateur, y compris l'installation des
outils nécessaires dans cette instance. Le nom initial `AI for All` a été refusé
par WSL avant création ; `AIForAll` a ensuite été explicitement choisi.
Le [contrat de rattachement](contracts/k6-chromium-causal-observation.md) distingue
le provisionnement Accepted de la qualification et des relevés causaux privés.

## Utilisation

Depuis Windows, toujours cibler cette instance explicitement :

```powershell
wsl --distribution AIForAll
wsl --distribution AIForAll --user root
```

La deuxième commande sert à l'administration de cette seule instance. Aucun mot
de passe ou secret n'a été créé/publié par le bootstrap, et aucune règle sudo sans
mot de passe n'a été ajoutée. Les vérifications k6/Chromium tournent comme
`aiforall`, jamais en root.

Les builds/tests Rust de développement restent natifs Windows avec le target E:
et l'unique slot Cargo. Aucun Rust ou Docker n'est installé spontanément dans WSL.
Les données nécessitant les garanties POSIX vont dans le HOME Linux natif, pas
dans `/mnt/c` ou `/mnt/i`. Le dossier et le VHDX Windows ne sont accessibles qu'au
propriétaire Windows courant, à SYSTEM et aux administrateurs ; le parent I:\WSL
préexistant n'a pas été modifié.

## Cycle de vie et conservation

L'instance, ses outils et les données de projet sont conservés sur I: conformément
à la demande. Son runtime reste à la demande : le parent termine/joint ses opérations
et ses enfants, libère les baux puis arrête seulement AIForAll quand elle est
devenue inutile. Aucun arrêt WSL global, défaut changé, unregister/reset ou
suppression n'est admis implicitement ; ne pas toucher aux autres distributions.
Le backend WSL partagé peut rester actif pour Docker Desktop : ne pas revendiquer
la libération de toute sa mémoire après un arrêt ciblé.

## Provenance et état de vérification

Image officielle acquise : Ubuntu24.04.5 amd64,
`https://releases.ubuntu.com/24.04.5/ubuntu-24.04.5-wsl-amd64.wsl`,388975696octets,
SHA256 `bb415d824822c4b878125729af451a5d18fb13d1cf5cbed9a7393ad64ac6039e`,
recoupé dans le catalogue WSL et les SHA256SUMS Ubuntu, puis vérifié avant installation.
L'image est conservée dans le cache d'acquisition dédié sur I: ; aucune copie de
ARTIST-Ubuntu, aucun WIP utilisateur ou ancien bail n'a été importé.

La création, le disque, le compte et les28paquets demandés sont vérifiés ; apt
update/simulation/install réussissent depuis les dépôts Ubuntu signés. Cela inclut
Python3, Git, curl, binutils et les bibliothèques/fonts Chromium, sans toolchain
Rust, Docker ou nouveau framework. Une campagne locale ultérieure a réellement
exécuté les140tests Python sans skip, acquis les deux outils épinglés et vérifié
leurs versions ainsi que les deux inspections. Le navigateur échoue avant le
cas ; un unique direct rapporte un format fatal/SIGABRT, sans cause identifiée.
Les enfants sont nettoyés et le runtime arrêté de façon ciblée, disque/outils
conservés. Voir [résultat local](../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/46-wsl-eprouve-mais-cause-native-encore-inconnue.md).
Les preuves locales ne qualifient ni le navigateur ni HTTP ou la cause GitHub.
Un relevé causal privé nécessite son admission et sa QA propres ; aucun relevé
de site inconnu n'a été créé.

Enquête ultérieure sans navigateur : une commande userns réussit réellement dans
le contexte Child, sous Ubuntu24.04/noyau6.6.87.2, seccomp0/AppArmor0. Une commande
ldd retourne0 mais son compte demeure inconnu ; lecture statique du même ELF :
32besoins directs, dont le chargeur, ce qui expose une asymétrie du lecteur de
résultats à corriger. Cela ne démontre pas une dépendance absente ou la cause de
l'arrêt Chrome. Les copies sont retirées et les baux libérés après observation
de cette seule instance arrêtée, sans toucher Docker ou ARTIST. Voir les
[mesures](../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/71-userns-reussit-dependances-encore-inconnues.md)
et [la limite du lecteur](../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/72-chargeur-direct-et-defaut-du-lecteur-demontre.md).

Une vérification indépendante fresh du2026-10-30 confirme ces faits, les ACL du
dossier/VHDX, le défaut WSL conservé et les versions des28paquets, à la portée
**provisionnement seulement**. Aucun navigateur ou test fonctionnel exécuté par
ce verifier ; historique d'ACL du parent I:\WSL non démontré faute de snapshot.

Versions observées des outils système : Python3 `3.12.3-0ubuntu2.1`, Git
`1:2.43.0-1ubuntu7.4`, curl `8.5.0-2ubuntu10.16`, binutils `2.42-4ubuntu2.10` ;
libnspr4 `2:4.35-1.1build1`, libnss3 `2:3.98-1ubuntu0.3`, libasound2t64
`1.2.11-1ubuntu0.3`. Le manifeste complet reste dans les preuves locales de
provisionnement ; les binaires k6/Chromium épinglés ont été acquis séparément puis
contrôlés lors de la campagne locale décrite plus haut.

Inventaire contemporain/bail local : `.tmp/k6-dedicated-wsl-20261030-mvuqe1yi.json`
et `.tmp/wsl-resources-20261030.log`. État durable et commandes vérifiées :
[ledger de mission](../.cursor/handoffs/20261029-k6-01-socle.md).
