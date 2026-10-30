---
reviewer: security
persisted_at: 2026-10-30T20:27Z
reviewed_scope: k6-windows-r1-trust
briefing_commit: 9ef802f
code_reference: bd7a2bf58d7e079c5dec2fe141838255f29efa29
native_candidate: result_mvvf7u6v_0000_hvh0gc
security_decision: CLARIFICATION_COMPATIBLE_WITH_CONDITIONS
finding_r1: OPEN
live_run_admission: FROZEN
---

# R-1 — Avis ciblé sur la frontière de confiance

## Conclusion

**La voie proposée est compatible avec le modèle de menace initial**, sous conditions explicites de protection persistante et de conservation. Elle n’exclut pas nouvellement l’attaquant de R-1 : celui-ci était un acteur moins privilégié pouvant modifier une DLL préexistante, **pas un opérateur ou administrateur compromis**.

Je ne maintiens donc pas une obligation universelle de conserver des handles après la disparition de leur détenteur pour résister aux quatre rôles déjà fiables. En revanche :

- une ACL persistante ne prolonge pas les handles ;
- la confiance accordée à un rôle n’autorise pas à ignorer une mutation **accidentelle** ou une réutilisation prématurée ;
- des ACL actuellement restrictives ne révoquent pas nécessairement des capacités accordées auparavant ;
- **R-1 reste OPEN**, sans admission de run réel, jusqu’à la disposition ratifiée et aux preuves ciblées.

Aucun nouveau HIGH/CRITICAL démontré sur les assets concernés. Ce constat n’est ni un override ni une fermeture anticipée.

## 1. Frontière initiale préservée

### Sources

Au snapshot initial `68631c0`, déjà lu lors de la première revue :

- `windows-native.cs:115` déclarait l’opérateur, SYSTEM et les administrateurs fiables, sans protection « anti-admin DRM » ;
- `Trusted`, ligne127, exemptait déjà exactement le SID de l’opérateur, SYSTEM,
  Administrateurs et TrustedInstaller.

Le recoupement historique communiqué par le lead pour WINDOWS.md:65–68 concorde
avec ce code. Aucune commande n'a été relancée pour relire cet historique.
Dans bd7a2bf, les rôles restent les mêmes, windows-native.cs:121,135–144. Aucun
SID supplémentaire n’est rendu fiable. Le contratAcceptedWindows:86–98 conserve
propriété des processus, jonction et verdict nonpositif si cleanup nonprouvé ;
la clarificationACL n'en dispense pas.

### Mini threat model R-1

- Asset : intégrité du code chargé par les outils sous le compte opérateur.
- Attacker : principal nonfiable ayant un accès limité à un fichier préexistant,
  sans autorité opérateur/SYSTEM/Administrateurs/TrustedInstaller.
- Entrée : ouverture ultérieure d’une DLL.
- Frontière : fichier modifiable par ce principal → code exécuté sous l’opérateur.
- Abus initial : DLL altérée alors queZIP et exécutable principal restent authentiques.
- Mitigations : bundlefermé comparé auZIP authentifié, contrôles individuels
  propriétaires/ACL/ancêtres, reparsesrefusés et leases pendant leur durée effective.
- Assurance aprèsrelease : droits persistants empêchant le principal d'écrire,
  remplacer ou reprendre le contrôle des objets protégés.

Assurance plus étroite qu'une immutabilité contre rôlesfiables, mais correspondant
à l'attaquant initial.

## 2. PASS A — Ce que les ACL peuvent réellement garantir

Open-K6BundlePins, windows.ps1:133–199, valide chaque fichier extrait et répertoire.
PathLease.Acl, windows-native.cs:136–144, vérifie propriétairefiable, absence de
droitsdangereux accordés auxSIDnonfiables, protection des fichiers/répertoiresprivés
et ancêtres contre suppression, changementdepermissions, prise de possession et
modificationdesattributs. Les descripteurs appartiennent aux objetsfichiers :
fermer unhandle ne les supprime pas.

**Si objets et ancêtres sont effectivement protégés et demeurent dans cet état**,
fermer les leases ne redonne pas le droit d'écriture refusé à l'attaquant initial.
Close-K6PinnedTool:216–218 et son appel inconditionnelrun-windows.ps1:94 libèrent
réellement les protections de partage. Aucun texte ne doit affirmer leur prolongation
aprèsrelease ou fin duhost.

## 3. PASS B — Défaillances à ne pas exclure

### A. Mutations accidentelles sous un rôle fiable

Un opérateur compromis est horsfrontière initiale ; nettoyage, réextraction ou
maintenance légitimes lancés trop tôt ne le deviennent pas automatiquement.
**Bundle et données réservés, sans mutation ni réutilisationmutatrice tant que
la disposition manque.** NonPASS, bailexpiré, PIDabsentd'unjournal ou nom ne prouvent
pas qu'une mutation est sûre. Cela ne rouvre pasS-1, corrigé séparément.

### B. Capacités antérieurement accordées

Une DACLrestrictive actuelle ne révoque pas tous les handles précédemment accordés.
Un principal nonfiable ayant obtenu avant sécurisation unhandle permettant de
modifier lespermissions pourrait conserver cettecapacité après resserrement deDACL.
Aucune exposition des assetsactuels n'est affirmée. Ceprincipal ne devient pas
« opérateur compromis » par cetteclarification.

**Assurance fondée sur une provenanceprivée connue des objets**, pas requalification
silencieuse d'un ancienbundle exposé par simple resserrementACL. Histoireinconnue
reste unelimite àtraiter ; ni mutation ni réextraction des assetsconservés demandée.

### C. Compteur zéro et journal manquant

Invoke-K6Child : créationChildwindows.ps1:246, incrémentExecuted:247, écritured'observations:253.
Une interruption/échec entre cesétapes interdit « Executed0 ou journalabsent ⇒
aucunprocessuscréé ». La conservation couvre démarragerefusé, étatinconnu et fin
duhost, sans dépendre uniquement du journalpostcréation. State.Cleanup est déjà
false avantconstructeur : conserver cetteprudence dansladispositionparent.

### D. Séparation entre sorties et bundle

« Le runner ne modifie jamais sonbundle » nécessite une garde/précondition vérifiée.
run-windows.ps1:32–39 vérifie sortiehorsGit et parentprivé, pas explicitement une
sortieneuve à l'intérieur d'un répertoirebundle. Lignes46,51–59 y écriraient bail
et copies de sources aprèslepréflightfermé.

Pas nouvelleprivescdémontrée, mais conditiondecohérence de conservation/ensemblefermé :
**interdire sortie/profil dans le bundle protégé**, mêmesgaranties dechemins et
identités, pas un préfixe ambigu.

## 4. Gardes minimales

1. Assurance nommée : protectionpersistante contre principauxnonfiables, pas
   immutabilitéuniverselle ou prolongationdeshandles.
2. ACLindividuelles/ancêtres contrôlés inchangés, aucun assouplissement.
3. Provenanceprivéeconnue, pas révocationhistorique supposée parACLactuelle.
4. Conservationparent : inconnu/survivant/finduhost ⇒ pasmutation/réutilisation
   mutatrice avant dispositionvérifiée. Ledgers/préconditionsexistants possibles,
   pas infrastructure supplémentaire imposée.
5. Sortiesdisjointesdubundle, vérifiées avant création/écriture.
6. Verdictsindépendants : R-1fermé ne prouve pas extinction, cleanupPASS ou R-2.

## 5. Oracles ciblés encore nécessaires

À planifier aprèsratification, non exécutés ici :

- DLLauxoctetsauthentiques mais propriétaire/droitsnonfiables ⇒ refus avantChild ;
  données, permissions/propriété et suppressionviaancêtre couverts.
- Aprèsrelease : descripteursfixtureinerte identiques et conformes. Ne pasattendre
  un refusd'écriture de l'opérateur fiable lui-même.
- Provenanceinconnue : pas assurancepositive depuis booléendéclaratif sanspreuve
  decréation/protectionprivée.
- Fininconnue : cleanupfaux, journalmanquant, compteur0 aprèscréationpossible,
  interruptionhost/bailexpiré ne rendent paslamutationadmissible.
- Chevauchementoutput/bundle : refusavantécriture, inventairebundleinchangé.
- Cheminnominal : dispositionpositive depuisobservationsautorisées, sansadoption
  PID/nom et sanschanger laportéedesautresverdicts.

Préflightréel et testsprécédents restent utiles, pas preuvesseules deprotection
aprèsrelease ou dispositiondesurvivants.

## 6. Limites et statut

Lecturesseules : briefing9ef802f, avisarchitecte, extraitsNative/PS/callsites et
contratAccepted. Aucune commande/hashsupplémentaire/test/runtime/permission/édition.
Empreintesbd7 précédemmentvérifiées restent références, non recalculées ici.

**Clarification recevable sousconditions ; aucun Proposed présuméAccepted ;
R-1OPEN et vraisrunsgelés** jusqu'àratification, preuvesciblées etrevuedufragment.

Ne pas rouvrirS-1, préflightR-1 ou fauxPASSR-2 ; aucune promessed'immuabilitéuniverselle,
réparationACLexistante, réextractionassets, nouvelleinfrastructure/admissionimplicite.
Reprise : dispositionratifiée ACL/handles distincts, conservationnonmutatrice vérifiée,
oraclesciblés puis revue ducontenuintégré.
