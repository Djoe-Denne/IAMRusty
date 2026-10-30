# Instance créée sur I: ; compte privé vérifié

## Livré et vérifié

**AIForAll est enregistrée sur I:** avec un GUID nouveau et un disque ext4.vhdx
dans le dossier demandé. Le nom technique choisi a été accepté. Le défaut WSL et
les anciennes distributions n'ont pas changé.

Le compte de travail `aiforall` est créé et réellement utilisé par défaut, avec
UID1000 non-root et HOME privé0700 sur ext4. Le dossier Windows et son VHDX neufs
sont limités au propriétaire courant, à SYSTEM et aux administrateurs ; le parent
préexistant I:\WSL n'a pas été modifié. Les outils seront administrés dans cette
seule instance, sans mot de passe publié ou sudo sans mot de passe ajouté.

## Étape en cours et limites

L'installation des outils utiles et bibliothèques Chromium est **réussie** depuis
les dépôts Ubuntu authentifiés, sans nouveau dépôt, toolchain Rust ou Docker.
La procédure vérifie les sources/noble avant apt, puis simule et installe sans
paquets recommandés. Refresh, simulation et installation sortent0 ; les28paquets
demandés sont vérifiés par dpkg, sans prétendre qu'il s'agit de tous les paquets
transitifs installés. Aucun Chromium ou k6 n'est invoqué par cette opération.

L'instance et ses outils resteront conservés. Le runtime est possédé par la mission
pendant son provisionnement actif, puis sera arrêté de manière ciblée quand les
consommateurs seront terminés. La qualification navigateur et le relevé privé
restent distincts ; les preuves GitHub ne deviennent pas une preuve de cet hôte.

## Référence d'utilisation

Le [guide de l'instance](../../../../docs/wsl-aiforall.md) décrit l'accès explicite,
la séparation administration/non-root et le cycle de vie. Aucun GO de jalon n'est
demandé : le lead poursuit les installations déjà autorisées puis les vérifications.

## Annexe technique

- Création native exit0, GUID `{22ce910b-45a3-42a4-bbab-dc56a1766035}`, BasePath
  I:\WSL\AIForAll, WSL2 ; disque initial1421869056octets, sans lancement automatique.
- Bootstrap non interactif via GUID : Ubuntu24.04, Python3.12.3, useradd1000 puis
  `wsl --manage AIForAll --set-default-user aiforall`, exit0. Invocation ultérieure
  sans option--user : UID/EUID1000, gid1000, HOME700/owner1000, rootFSext4.
- Registre contemporain append-only avec GUID réel, bail actif de provisionnement ;
  ACL Windows restreintes uniquement sur les deux nouveaux objets vérifiés sans lien.
- Script temporaire relu/AST valide, SHA256
  `23da9997e786de8d555172b7b00b48b82ffdaf6192d245bb4747f0d65178218e` ;
  opération package achevée exit0, manifeste fermé relu, stderr vide. Origines
  archive.ubuntu.com et security.ubuntu.com, logs de paquets bruts non publiés.
