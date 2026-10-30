# Formaliser la protection persistante dans la frontière de confiance existante

Architecte12, continuation même session/profil/modèle. Avis précédent reçu et
persisté dans20261030-k6-windows-bundle-lifetime-result.md. SourceOPSbd7a2bf,
rapportindépendantcomplet20261030T2005Z-security-k6-windows-delta-bd7a2bf.md ;
politique/contratsWindowsAccepted inchangés. Vraisrunsencoregelés.

## Choix proposé par le lead et preuve de continuité

Retenir la première assurance si votre recoupement confirme qu'elle respecte la
frontière initiale : protection contre les acteurs non fiables, pas contre un
opérateur/administrateur compromis. Aucun SID fiable nouveau ou permission nouvelle.

Le lead a vérifié AU COMMIT68631c0 : WINDOWS.md65–68 indique déjà explicitement
« does not defend against a compromised operator or administrator » ; C#115
« The operator, SYSTEM and Administrators are trusted; this is not anti-admin DRM » ;
Trusted127 énumère déjà opérateur/SYSTEM/admins/TrustedInstaller. R-1initial décrit
un acteur moins privilégié disposant d'un write sur une DLL préexistante. La liste
Trusted est inchangée. Ne pas confondre S-1concurrenceaccidentelle de deux créateurs
sousmêmecompte (corrigéeatomiquement) avec un mutateurdecodeopérateurcompromis.

Ces éléments code/docs ne dispensent pas d'examiner la cible Accepted : si elle
exigeait une assuranceplusforte, le signaler au lieu de la dégrader. Aucun changement
de contratQA/pin/sandbox ou dérogationS1/R2 n'est proposé. Le reviewer21 vérifie
indépendamment enparallèle si un acteur couvert par son R-1serait perdu.

## Livrable documentaire minimal

Si compatible, écrire uniquement `docs/contracts/k6-windows-code-custody.md`
**Proposed**, petit addendum lié au contratWindowsAccepted. PasOPS/Accepted/
QAJSON/ledger ou anciensfichiers de votreworktree modifiés. Corpscompletnatif+SHA
et limites, pasackadditionnel. Le lead ratifie/intègre avant consommateur.

Expliciter assurance et acteurs, validationavantusage archive+ensemblefermé+octets/
ACLindividuelles/noreparse/identités, protectionsnormales parhandles et garantie
persistante exacte aprèsrelease/findhôte. Ne pas appeler cela immutabilité générale.
Conserver bundles/données si survivant ou finnonprouvée, aucune mutation/réextraction/
remplacement/réutilisationmutatrice avant dispositionréellementvérifiée ; ancienbail
ou expiration ne prouve jamais absence. Pas nouvelleautorité de cible ou service
de stockage : les modes actuels sont déjà readonly sur le bundle, et les sorties
restent privées/neuves/atomiques. Indiquer le minimum concret manquant si cela ne
suffit pas à respecter cette réserve.

Définir l'oracle ciblé nécessaire : état de protection persistante et identités
avant/après libération/fin du host, avec enfantconnu à durée naturellebornée au-delà
de la jonction ; refus d'ACLindividuelleunsafe sur donnéeinerte créée pourfixture,
sanschargement deDLL. Réutiliserfixtures natives existantes, pas nouveauframework,
autre superviseur de production, privilège, modificationACLexistante ou sondetokenR2.
Ne pas prescrire une expérience impossible sans nouveaux droits ; préciser son
caractère de simulation/contrôle et limites. Les22preuvesvalidesrestent àleurportée.

Si la première assurance demeure insuffisante, rendre la raison précise et plus
petit choix conforme restant ; ne pas inventer un GO d'exécution ou une exception
de contraintes. Documentation/recherche seulement, aucuntest/Windows/Chrome/WSL/
Cargo/installation/runtime, pas de nouvellecampagne.
