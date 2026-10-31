# Explication : fichiers protégés, processus survivant et isolation Chromium

L'utilisateur demande de préciser le blocage et la raison du sandbox. Ce document
sépare le risque de cycle de vie du lanceur de la qualification de l'isolation.
**Aucun k6 ou Chrome de la campagne Windows n'a encore été lancé. Aucun Chrome
survivant ou fichier réel altéré n'est actuellement observé par cette campagne.**

## Fichiers et processus concernés

Ce sont les fichiers de la distribution portable Chrome for Testing156, pas le
Chrome149 installé ni les services AIForAll. L'archive officielle vérifiée contient
notamment `chrome-win64/chrome.exe`, `chrome-win64/chrome.dll` et
`chrome-win64/chrome_elf.dll`. Les DLL sont des bibliothèques contenant du code
chargé par le navigateur ; vérifier seulement l'exécutable ne suffit donc pas.
Le préflight actuel compare déjà tout le lot à l'archive authentifiée.

Chaîne prévue : notre processus powershell.exe exécute run-windows.ps1, crée
k6.exe, lequel lance plusieurs chrome.exe, dont le renderer qui exécute la page.
« Survivant » désigne un enfant de cette future chaîne encore vivant, ou dont la
fin n'est pas établie après l'attente bornée. Ce n'est pas un processus utilisateur
existant identifié sur la machine ; il n'y a pas de PIDChrome de cette campagne.

## Protection temporaire et permissions persistantes

Le lanceur garde des handlesWindows, références ouvertes sur les fichiers et
dossiers. Leurs modes de partage interdisent les écritures/remplacements concurrents
couverts pendant leur durée de vie. Ils sont distincts des permissions persistantes
du disque, les ACL, qui contrôlent quels comptes peuvent accéder aux objets.

Chemin d'échec précis relu dans le code intégré :

1. Child.Finish peut rendre Cleanupfalse si un processus survit ou si la preuve
   de fin est incomplète, sans recours à une terminaison forcée.
2. run-windows.ps1:94 appelle quand même Close-K6PinnedTool dans finally.
3. windows.ps1:216–218 ferme les références protégeant lot/exécutable/archive.
4. La sortie du processusPowerShell les fermerait aussi. Elles ne protègent donc
   plus une DLL qui pourrait être chargée ultérieurement par un enfant survivant.

Cela ne rend pas les fichiers accessibles à tout le monde : leurs permissions
Windows restent en place. La disposition en cours consiste à prouver leur protection
persistante contre les acteurs non fiables, dans la frontière déjà déclarée, et
à réserver code/archives/données tant que la fin manque. Aucune garantie contre
opérateur/administrateur compromis n'est ajoutée.

Le blocage est donc une condition d'admission de **notre lanceur de test**, pas une
panne démontrée d'AIForAll ou de Chrome. Derniers points à fermer : provenance privée
connue, sorties/profils hors du lot d'outils et conservation non mutatrice sur fin
incertaine. Une maintenance légitime trop précoce ne doit pas modifier ce lot.

## Sens du sandbox Chromium

Le renderer est le processus qui traite la page et son JavaScript. Le sandbox
réduit ses droits système afin qu'une faille de ce processus ne donne pas directement
les droits ordinaires du compte utilisateur. Sous Windows, cela repose notamment
sur des jetons de sécurité et niveaux d'intégrité restreints. Pas une VM, Docker
ou WSL ; le navigateur principal et ses processus n'ont pas tous le même rôle.

À distinguer : headless signifie sansfenêtre ; contexte/profilneuf sépare cookies
et données du test ; sandbox limite les droitsOS ; JobObject sert ici au suivi
des processus et au cleanup, pas à prouver ce confinement.

Le témoin actuel ne lit que le jeton principal d'un renderer. Chromium peut déjà
avoir ce jeton restrictif pendant que son thread initial utilise temporairement
un jeton plus puissant pour démarrer. L'échantillon n'est pas non plus lié avec
certitude au renderer et au moment qui ont exécuté notre cas. Cela ne prouve ni
sandboxdésactivé ni évasion ; cela interdit simplement de certifier son état
effectif complet à partir de ce seul témoin.

## Nécessité proportionnée à ce test

Un GET via k6/http n'utilise pas Chromium : aucune isolationChromium n'est
nécessaire pour ce seul testHTTP. Pour about:blank et le calcul42, le risque est
bien plus faible que lors d'une navigation sur un site applicatif. La preuve complète
du sandbox n'est pas une nécessité technique pour faire fonctionner ce cas.

Le contrat de mission impose de **conserver** les protections normales et de ne
pas les désactiver pour obtenir un vert. Il permet déjà une exécution fonctionnelle
distincte tout en marquant ChromiumINCONCLUSIVE si la preuve manque. C'est la
disposition retenue, pas l'ajout d'un mécanisme de confinement supplémentaire.

Ainsi, le manque d'attestation complète du sandbox empêche un PASS de qualification,
mais **n'est plus à lui seul le blocage du lancement du cas minimal**. Le gel
actuel concerne l'admission des protections/conservation du code. Versions,
inspections, HTTP, JavaScript, sandbox et cleanup doivent garder des résultats distincts.

## Sources revérifiées pour cette explication

- run-windows.ps1:65–112 ; windows.ps1:205–272 ; windows-native.cs:388–432.
- MétadonnéesZIPofficiel relues sans exécution, présence des trois fichiers cités.
- Avisarchitecte et revues déjà archivés : rapports85,88,89,91. Pas de nouvelle
  expérience ou validation comportementale ; aucun k6/Chrome/HTTP/WSL lancé.
