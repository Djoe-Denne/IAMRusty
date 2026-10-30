# Revue indépendante bornée des frontières Windows

## But et périmètre indépendant

Contexte fresh, security-reviewer, lecture seule, sans héritage auteur.
Canon C:/Users/djden/source/repos/AIForAll, HEAD68631c0 intégré avant admission.
Révision native2 approuvée, QAproportionate et catalogueOpenAI. Pas de pipeline
supplémentaire implicite ; cette revue vise les nouvelles frontières de confiance
Windows, pas la réécriture du harness ou la qualification Linux.

Revoir les primitives Windows et leurs vrais callsites au commit68631c0 :

- `ops/tests/k6/lib/windows-native.cs` SHA256
  82b00f61fabc697f2872605033eead985fbd49fdfb1005d34b70692461bdc2dc :
  PathLease/Native/Child/RendererToken/JsonGuard, handles/processus/job/token,
  descendants/interruptions/arrêtgracieux, observation et capture bornée.
- `lib/windows.ps1` : Open-K6PinnedTool, Invoke-K6Child, Remove-K6OwnedTree et
  gardes de confidentialité/identités/pins. `run-windows.ps1` : création privée,
  environnement/callsite, disposition des captures et gardes de sortie.
- `config/tools.windows.lock.json` et testsWindows aux frontières pertinentes,
  scénarios existants pour flux admis. Pas besoin de relire toutes les suitesLinux.

Le C# et les helpers processus/chemins restent gelés pendant cette revue. L'auteur
travaille indépendamment sur le raccord de version décrit dans le contrat dédié,
principalement environnement/validation de résumé et checkJS. Lire au commit68631c0
si un fichier change ; noter les hashes examinés et tout delta hors périmètre.
Les nouveaux changements seront intégrés puis leur impact revu par le lead ; ne
pas transformer cette revue partielle en approbation d'un futur diff inconnu.

## Contrats et invariants

`docs/contracts/k6-windows-functional.md` Accepted et
`docs/contracts/k6-windows-browser-version.md` Accepted, lus au HEADci-dessus.
ADR0504/V1Linux gardent leur portéeLinux et suspension des deuxjobsCI obligatoire
avant publication ; ce n'est pas une preuveWindows. Mission1 seulement : outils,
seul contexteabout:blank/42, GETIAMpublic conditionnel ; aucune autoritécibleadmise
dans l'état présent. Pas d'OAuth/POST/charge/seed/Rust/service/identité.

Exigences : compte ordinaire, pinsavantuse et pas fallback149, chemins privéshorsGit,
ACL/ancêtres/reparse/identité retenue ; pas adoption nom/PID ou ancienbail. Processus
affectés avantexécution/descendants sanséchappement, fin par mêmehandle ; pas
killonclose/taskkill/force/admin/ACLmutation existante. Témoinrenderer effectif selon
politiqueChromium156, pas simpleJobObject. Unknown/zero/summaryabsent/stale ou
cleanup/privacy manquant => nonvert. Captures bornéesprivées, sortie fermée sans
exceptions/logs libres/sentinelles/credentials/URLs/chemins/SID natifs publiés.

## Méthode et livrable

Deux passes, défensive puis adversariale, preuvesancrées/codepaths exploitables.
Pas d'édition, test, binaire k6/Chrome, fixtureprocessus, HTTP, Docker/Kind/WSL,
installation, permissionmodification ou nouveau harness. Sources officielles
Windows/Chromium/k6 versionnées admises si nécessaires. Pas de conclusion de
preuve fonctionnelle ou de sandbox réel à partir de ce seul auditstatique.

Retour natif substantif : findingsbloquants CRITICAL/HIGH séparés des suggestions
et gaps de preuve/outofscope. Décrire condition/conséquence/callsite/régression
minimale, pas seulement un principe générique. Absence de finding démontré reste
bornée au périmètre lu. Localisation/Impact/Limites, fichiers/hashes/sources et
commandes réellement faites. Lead persiste rapport avant toute correction ;
ne modifier ni ledger partagé ni sources. Pas de conclusiondu développeur fournie.
