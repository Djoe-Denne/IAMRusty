# Démarrage approuvé du raccord Windows

L'utilisateur a approuvé la révisionWindows. Le but est maintenant de vérifier
k6 et Chrome natifsWindows, puis le cas navigateur minimal, sans repriseWSL.
La preuveLinux et la future machineGitHubActions restent une mission distincte.

## Travail engagé

Le contratWindows de l'architecte est ratifié avant code. Un auteur isolé réalise
le petit lanceur et ses testsWindows. Enparallèle, le lead vérifie les octets
officiels des outilsWindows et prépare la suspension explicite des deuxjobsLinux
avant toutpush. Ces opérations sont indépendantes et leurs fichiers n'ont pas
deux propriétaires.

Les preuves réelles attendront l'intégration du lanceur, du verrou de versions et
les vérifications de sûreté nécessaires. Aucun PASS n'est présumé : la réussite
duJavaScript, les versionsoutils, l'isolationChromium et le retrait des processus
auront leurs verdicts séparés. HTTP restera inconclusif sans cibleexistanteadmise,
sans nouvelle stack, tunnel ou requête de contrôle déguisée.

WSLreste suspendu et inconclusif. Les19sources archivées restentinactives ; le
dernier raccordldd inachevé ne sera ni consommé ni repris pour achever l'archive.
Les fichiersutilisateur, le sous-module et les travauxétrangers sont préservés.

## Contrôles prévus

QA proportionate : tests ciblés sur les nouveaux risquesWindows, versionsréelles,
deuxinspections puis casabout:blank sous les protections existantes. Une revue
indépendante sera bornée aux risques réels nouveaux du diff intégré, pas une
chaîne de quatreagents. Publication seulement après suspensionLinux et contrôle
des artefacts ; aucun dispatchGitHub ou réveilDocker/Kind/WSL.

## Annexe de suivi

- Approbationnative : révision2, hash
  `4d651a945ab4f6757f24117ed715c37cf82fc712bf5b22721d593240543cac64`.
- Contrat `docs/contracts/k6-windows-functional.md` Accepted/Unimplemented,
  sourcearchitecte comparée32bb3c… avantratification.
- Brief complet `20261030-k6-windows-implementation.md`, intégré avant auteur.
- Entréeconvenue `ops/tests/k6/run-windows.ps1`, PowerShellWindows5.1 ;11commandes
  et18critères de la politiqueapprouvée, catalogue/QA/autonomie inchangés.
- Aucune nouvelle compilation, exécutionk6/Chrome/HTTP ou qualification réelle
  n'est acquise par ce rapport de démarrage.
