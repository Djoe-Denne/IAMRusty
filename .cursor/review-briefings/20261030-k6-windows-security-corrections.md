# Correctifs Windows avant activation réelle : création et code chargé

Continuer auteur20, même tâche/session/profil/modèle. ContratsWindows fonctionnel
et versionAccepted, révision2/QAproportionate inchangée. Lire le rapport indépendant
complet `20261030T1913Z-security-k6-windows-primitives-cbcc1bb.md` et cet index
avant correction. Le rapport et les décisionslead sont persistés avant ce retour.
Source gelée examinée68631c0 ; HEADcanonique fourni dans le message de reprise.
Vos changements de version en cours restent à livrer/recouper, pas annulés.

## Décisions lead et ownership

Les vrais k6/Chrome/HTTP restent interdits au worker et gelés au lead jusqu'aux
conditions de revue. Code, tests et recherches de détail restent autorisés.
Le C# est maintenant dégelé **seulement** pour corriger création atomique/liaison
de propriété de S-1 ; l'examen figé du reviewer est terminé. Pas de mutation du
superviseur Child/Job ou d'élévation. R-1 concerne vos helpers de pins/interop
locale minimale et callsites ; aucun lock modifié ou nouvelle valeur inventée.
Le lead garde provenance/archives/lock/workflow. Aucun autre écrivain de ces sources.

### Dossier : création exclusive et nettoyage lié à l'identité

Corriger S-1 avant même tools : création native atomique du seul répertoire final
avec sécuritéprivée à la création, refus si déjà présent, aucune mutation d'ACL
existante. Le contrôle préalable Directory.Exists n'est pas une création exclusive.
L'autorité de suppression exige succès effectif de création ET identité retenue/
liaison valide. Échec de liaison => nonvert et préservation, pas adoption d'un
gagnant concurrent. Le nettoyage ne doit jamais perdre son lien à cette identité.
Ne pas créer un service d'autorité de fichiers ou une nouvelle API publique.

TestROUGE discriminant avantfix quandpossible : deux créateurs synchronisés,
gagnant crée son témoin, perdant refuse sans modifier/parcourir/supprimer le
contenu étranger. Tester aussi échec de liaison après création, chemin substitué
avant cleanup et création nominale/reuse. Fixtures jetables possédées, aucun
répertoire utilisateur, aucune ACL ancienne changée, pas de forçage.

### Code Chrome : couvrir les dépendances réellement chargeables

R-1 devient HIGH si le refusversion est levé sans correction. Authenticité du ZIP
et de chrome.exe ne suffit pas pour chrome.dll et le codeextrait. Établir un ensemble
fermé depuis l'archive authentifiée, vérifier octets et chemins/ACL/nonreparse/
identités de ce code puis retenir la nonmodification jusqu'à la fin de tous les
processus concernés. Réutiliser les pins/PathLease existants ; pas ajout de schéma
auxiliaire gratuit, nouveau framework ou extraction/install automatique au run.
Une énumération bornée du bundle authentifié et comparaison des entrées extraites
peut être plus simple qu'un nouveau manifeste public ; choisir le détaillocal
minimal après lecture des API .NET5.1/Windows officielles. Refuser entrées ambiguës,
collisions et dépendance absente/substituée avant toute création de processus.

Test au vrai préflight/callsite avec fausse dépendance jamais exécutée : exe et
archiveauthentique connus inchangés + dépendancefacticesubstituée doit refuser.
Pas de DLL hostile fabriquée ou chargée, pasvraiChrome pour tester ce rejet.

### Isolation : ne pas rendre la preuve primaire plus forte que les faits

R-2 confié en parallèle à l'architecte12 pour recherchebornée. Le booléen actuel
Sandbox ne suffit pas à qualifier le cas : pas de PASS depuis un seul primaire
restreint échantillonné ou un autre renderer. Jusqu'à décision compatible vérifiée,
le parent doit conserver Chromium INCONCLUSIVE même avec JS/version réussis.
Ne pas implémenter par anticipation une enquête threads/token ou instrumentation
nouvelle ; ne pas contourner l'accès aux tokens avec privilèges ou ACL.

## QA et livraison

Réutiliser21preuves initiales dans leur portée ; tests ciblés sur vraiscallsite et
négatifs introduits, nouveauxguardsversion restentàvalider. Pas desuiteshistoriques,
serveur/WSL/HTTP/k6/Chrome/Cargo/install ou nouveau harness autonome. Documenter
ROUGE/GREEN réels, counts/exits/skips/cleanup des seulesfixtures connuesWindows.
Le test-engineer peut être demandé au lead si les tests exigés dépassent votre
tranche ; pas de délégation worker. La création et l'intégration desconsommateurs
restent au lead. Aucun modèle/QAoverride par message.

Rendre rapport substantif et deltaexact contre les7hashs initiaux + scénarioJS,
SHAfinaux et limites. Le lead persiste/diffcheck/intègre avant relecture ciblée21.
Si vous devez livrer d'abord le correctifversion déjà terminé, le geler et rendre
ses octets/hashes avant de poursuivre S-1/R-1, sans relancer les mêmes tests.
