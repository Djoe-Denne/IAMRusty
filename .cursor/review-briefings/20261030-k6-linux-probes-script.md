# Script ponctuel des sondes Linux — auteur existant13

## Contrat et résultat attendu

Même session/profil/modèle, aucun nouveau spawn ou changement de politique. Lire
**le canon principal actuel**, pas le contrat de votre ancien HEADb316 :
`C:/Users/djden/source/repos/AIForAll/docs/contracts/k6-chromium-linux-probes.md`
est Accepted par lead après reprise explicite utilisateur ; V1 contient le lien
d'exception. Source canonique HEAD0f32, diff actuel3correctifs conducteur gelé.
Les22fichiers existants restants sont byte-identiques195 ; vos3correctifs ont
passé194testsWindows/16.998s/exit0/skips8, revue19 en cours. Ne pas les modifier.

Écrire seulement **un script ponctuel** et ses fixtures unitaires pures sous votre
`.tmp`, hors Git ; aucun fichier ops, contrat, workflow, test durable, index ou
ledger modifié. Le lead copie/vérifie les octets puis exécute sous bail. Pas de
framework, superviseur alternatif ou mode permanent. Aucun WSL/Linux/Chrome/k6,
installation, Cargo, commit, push ou sous-agent par l'auteur.

## Fonctionnement borné attendu

1. Script Linux stdlib, UID/EUID1000, HOME réel privé/ext4, modeCI refusé, chemins
   privés/horsGit/non-substitués. Le lead fournira chemins de source snapshot et
   outils déjà présents après une lecture metadata bornée : **ne pas les deviner**.
   Paramètres d'installation ne sont jamais un argv libre ; commandes de sondes
   fixées par contrat uniquement.
2. Avant import/exécution, vérifier bytes des modules effectivement chargés contre
   hashes des fichiers canoniques fournis explicitement ; réutiliser le runner
   `pinned_tools`/`verify_tools` et les vrais `Child`/`child_environment` sans copie
   ni modification. Le snapshotLinux887b existe historiquement ; sa fraîcheur et
   ses fichiers seront revérifiés. Le runner charge safety/private_site/lock ;
   jamais main ou PrivateSiteSession. Hashes attendus : calculer depuis les
   quatre fichiers CANONIQUES, pas votre versionhistorique.
3. Répertoire de run privé neuf natifLinux ; cibles toolspinnées : archive+ELF
   Chrome conformes au lock, toutes vérifications existantes actives, **aucun
   --version**. Avantldd, vérifier ELF-interpréteur x86_64 attendu et chargeur
   système fiable (rootowner, pas écriture groupe/autres), comme outils système
   absolus déjà présents. Aucun loader libre ou binaire fourni arbitrairement.
4. Relever contexte du parent WSL puis contexte d'un enfant par Child : champs
   OSbornés, noyau, UID/GID, étatsproc/valeursusernsAppArmor et namespace-identités
   privés comparables. Seulement fichiers précis admis, pas dumpenv/proc global.
   Exporter états/chiffresbornés et égalités de namespaces plutôt que PIDs/inodes.
5. Recommandation minimale retenue du contrat : **commandes seulement voieChild**.
   `/usr/bin/unshare --user --map-root-user -- /usr/bin/true`, puis
   `/usr/bin/ldd <ELFChromevérifié>`, chacun UNE fois, timeout10s, browser=False,
   envfermé existant, noK6/proxy/LDhérité. Une observationcontextes n'est PAS un
   testunshareA/B ; Acommandenonmesurée resteINCONCLUSIVE/NOT_EXECUTED.
6. CapturesChild mémoire bornées privées et contrôleconfidentialité réutilisé.
   Aucun stderrtexte libre ou stdout natif exporté, aucune exceptiontrace brute.
   Parserldd strict/nonvide : chaque ligne interprétée, lignes manquantes=>compte,
   exit0mêmeavecnotfound=>absences ; blanc/vide/partiel/inconnu/UTF8invalide/
   overflow/exitnonzéro/violations=>countnull, jamais faux0. Pas noms/adresses/paths.
7. Enfants/descendants surveillés et joints par Child, identité nonobservée reste
   explicitementINCONCLUSIVE, pas délaiartificiel ni PASScleanup vide. Retirer les
   seuls fichiers privés créés par script aprèsanalyse, ycompriséchec/interruption,
   vérifierrestants ; pas suppressionsource/outils/installations. Résultats fermés
   séparés contexte/userns/ldd/privacy/cleanup, jamais HTTP/ChromiumPASS. La sortie
   JSONpublique est bornée/schemafermée ; pas chemin privé/PID/site inconnu.

## Validation et livraison

Compiler AST avant exécution. Fixtures pures Windows de la projection/du câblage :
étatabsent/refusé, exitnonzéro, lddmissingexit0, toutnormal, vide/invalide/partiel,
sentinelle/UTF8/overflow, timeout/interruption/cleanupmanquant/identitéabsente.
Réutiliser helpers/tests existants adaptés, pas nouveau mockHTTP/infrastructure.
Pas de suite complète194redondante si sourcegeléeinchangée ; testsduseulscript.
Le lead relira tout avantruntime. Donner fichiers, SHA256, commandes/totaux/exits,
limites et protocole d'invocation fermé ; geler le candidat. Si la sûreté applicable
refuse l'hôte, rendreNONEXECUTE et la cause, aucun contournementduconducteur.

Les sondes ne débloquent PAS la collecte privée ni ses ACL/stockage/rendezvous,
et ne certifient pas le sandboxChrome. Aucun nouvel essai Chrome/logging/tailraw,
pin.80/helper/sysctl/ACL, Linuxnatif ou Windowsmigration dans ce lot.
