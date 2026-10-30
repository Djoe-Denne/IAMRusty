# Question sécurité ciblée : le périmètre R-1 est-il préservé ?

Reviewer21, même session/profil/modèle, readonly et sansnouveautest/runtime.
Rapportcompletderélecture reçu result_mvvemxgb_001m_ynm8dy, persisté avecS1fermé/
R1préflightfermé/rétentionouverte/R2fauxPASSfermé ; vraislancementsencoregelés.

L'architecte12 explique une voie potentiellement conforme par protection ACL
persistante contre acteursnonfiables aprèsrelease, sans garantiecontreopérateur/
adminsfiables. Son avis complet est20261030-k6-windows-bundle-lifetime-result.md.
Il ne lève pas le gel et ne ferme pas R-1 par lui-même.

## Vérification précise demandée, pas réaudit

Le lead envisage de retenir cette assurance seulement dans la frontière déjà
explicite AVANTvotre premierfinding : WINDOWS.md AU COMMIT68631c0:65–68 exclut
opérateur/administrateurcompromis ; C#115énonce mêmeconfiance et127Trustedliste
déjà opérateur/SYSTEM/admins/TrustedInstaller. Aucun SID ou droit n'est ajouté.
VotreR-1initial cible un acteur moinsprivilégié ayant write surDLLpréexistante.

Dire si un acteur ou une défaillance effectivement couverts par cefindingserait
indûment exclu par cetteclarification. Distinguer protectionACLpersistante vérifiée
de l'immuabilitéuniverselle, et concurrenceS1accidentelle mêmecompte de la
compromissiondélibéréed'un rôlefiable. Ne pas réinterpréter uneattestation
cleanupfalse comme une prolongationdeshandles.

L'ensemblecode déjàvalidé est privé aveccontrôlesACLindividuels ; modesrunner
n'écrivent jamais son bundle. Si survivant/fininconnue, bundle et données doivent
être conservés, sansmutation/réextraction/remplacement/réutilisationmutatrice
jusqu'à dispositionvérifiée ; expirationbail/PIDnomne vautpasfin. Dire les gardes
minimales et oracles nécessaires si cette voie est conforme ; sinon citer le
contrat/acteur/callsite empêchant de la retenir. Pas réauditerS1/APIversion ouR2.

Architecteécrit seulement un Proposed enparallèle ; ne pas le présumerAccepted.
PasfermetureR1anticipée : après éventuelle ratification, correction/testsciblés et
fragmentintégré reviendront en revue si nécessaire. Pasexigence de nouvelleinfrastructure,
custodian, attenteindéfinie, force, élévation/ACLmutation ou autrecontrat de modèle.
Rapportnatifcomplet borné sources/code/threatmodel/limites, aucunackadditionnel.
