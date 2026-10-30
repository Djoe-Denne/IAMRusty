# Une mesure ldd supplémentaire après correction démontrée du lecteur

- Statut : **Accepted** ; Réalité : **Unimplemented pour cette exception**.
- Date : 2026-10-30 ; mission1 k6 ; source OPS gelée
  `f279e713079159766188fcbe5078d37217ca530d`.
- Sources : briefing `20261030-k6-one-additional-ldd-arbitration.md`, rapports71/72,
  briefing `20261030-k6-ldd-loader-parser-fix.md`,
  [sondes Accepted](k6-chromium-linux-probes.md) et [V1](k6-functional-linux.md).
  Les résultats documentaires récents sont lus aux chemins canoniques, pas supposés
  committés. Aucun Accepted, ancien résultat, OPS ou candidat préservé n'est modifié.

## Décision et autorité

Ratification ciblée du lead le2026-10-30, dans le mandat long run déjà approuvé.
Le document architecte livré dans `result_mvv8umt6_000q_earwag` a été intégré
octet pour octet avant ratification, SHA256
`cf9b42313709fe44c8d23c41ec98dc39335c1e852d48f5f6eb15aa2929d2a2fd`.
Le défaut a été reproduit avant correction et les27tests de la copie corrigée
passent, comme établi au rapport73. Cela admet seulement la préparation et la
vérification du sélecteur temporaire et de son quota, puis une mesure gardée ;
aucune nouvelle exécution n'est acquise par cette ratification. La proposition
Windows séparée reste Proposed et le conducteur privé reste non admis.

**Compatible dans le mandat délégué**, comme exception instrumentale unique,
explicite et non qualifiante à la règle « une commande par voie, sans retries ».
Le lead référence et ratifie ce contrat avant code consommateur/runtime ; pas de
nouveau GO utilisateur générique, ni autorisation de relancer le lot complet.

Le défaut démontré omet des noms couverts le basename du chargeur pourtant reconnu.
Les32DT_NEEDED et le chargeur direct rendent le défaut pertinent, pas causalement
certain pour la sortie passée effacée. Le run `02fce7bacbc8435293dbbbbebaad17bd`
reste ldd exit0/countnull et diagnostic incomplet. Aucun reparsing/reclassement
rétrospectif, reconstitution de flux ou réécriture de ce résultat n'est admis.

## Quota global et prérequis

- Une invocation ldd réelle est déjà consommée ; **au plus une supplémentaire**,
  donc **deux au total pour cette enquête**, tous baux/voies/noms de mode confondus.
  Unshare reste à une invocation historique, sans nouvelle commande userns.
  Changer de bail, nom ou hôte ne recrée aucun quota ; aucune boucle automatique.
- Avant tout appel : correction minimale démontrée ROUGE/GREEN sur fixtures connues,
  revue du delta par le lead et vérification des sources exactes retenues. Ne pas
  déclarer ces preuves acquises dans cet avis. Le correctif du parseur seul ne
  débloque pas l'exécution sans cette admission de la sélection consommatrice.
- Ajouter le basename seulement après une ligne de chargeur autorisée effectivement
  reconnue ; garder whitelist, chargeur unique, couverture et refus stricts. Ne pas
  remplir names depuis les attentes DT_NEEDED pour fabriquer une couverture complète.
  Fixtures : chargeur directement nécessaire avec zéro absence, absence réelle
  malgré exit0, cas antérieur inchangé, chargeur absent/inconnu/doublon et besoin
  non couvert ; conserver les négatifs signal/non-zéro/UTF8/overflow/confidentialité.
- Sous bail neuf, le parent réserve l'unique exception et la corrèle au run/source
  avant lancement dans ses preuves existantes. Tout appel réellement lancé compte,
  quel que soit son résultat ; un lancement incertain n'autorise pas un deuxième.
  Un refus préalable reste NOT_EXECUTED, sans relance automatique ou quota inventé.

## Raccord temporaire ldd-only et schéma honnête

Sélection fermée explicite **ldd-only**, uniquement dans l'outillage ponctuel sous
`.tmp`, relu/testé après admission. Aucun mode permanent, modification de
`ops/tests/k6`, autre superviseur ou élargissement de la grammaire. Exécuter le
seul `/usr/bin/ldd` déjà admis avec le même ELF vérifié ; mêmes pins, UID/instance,
Child/browser=False, child_environment, limites10s/captures et gardes d'identité.
Les préparations/retraits indispensables restent gardés ; ne pas appeler la voie
globale qui lancerait aussi unshare. Pas de chargeur lancé manuellement, Chrome/k6/
--version, nouveau système/hôte, réseau, installation, LD_DEBUG ou capture accrue.

Réutiliser les clés/types/allowlists du résultat fermé existant, sans format neuf :
- `unshare` vaut exactement `{"outcome":"INCONCLUSIVE","exit":null,"termination":"NOT_EXECUTED","missing":null}`.
  Le succès userns historique est référencé séparément par le rapport71, jamais
  copié dans cette exécution comme ZERO_EXIT, exit0 ou preuve fraîche.
- `a_commands` reste NOT_EXECUTED. Contextes/comparaison restent null s'ils n'ont
  pas été réellement observés maintenant ; aucun contexte historique recopié.
  Ne pas ajouter d'instrumentation pour remplir des champs facultatifs.
- `ldd` conserve outcome/exit/termination/missing : entier seulement après sortie
  courante complète, reconnue, non vacante et contrôles réussis ; sinon missing=null.
  Aucun nom de bibliothèque, chemin, adresse ou flux brut exporté.
- Le verdict du script porte sur la sélection ldd-only : unshare non exécuté n'est
  ni une réussite synthétique ni un échec requis de cette sélection. Conserver
  l'ancienne voie inchangée et tester que même les chemins négatifs n'appellent pas
  unshare. Le sélecteur reste explicite dans les preuves privées du parent.

**Codes de sortie :** préserver la distinction natif/script. Exit0 du script
seulement si ldd natif exit0, analyse complète, missing=0 et confidentialité/
cleanup/retrait requis prouvés. Si missing>0 est mesuré, garder ce compte positif,
ldd natif exit0/outcome ZERO_EXIT, mais script exit2/DIAGNOSTIC_INCOMPLETE : absence
de succès diagnostique, pas compte inconnu. Analyse inconnue, refus, signal,
interruption ou échec de garde => script non nul, jamais un zéro pour un skip.
L'ancienne condition globale exigeant unshare ZERO_EXIT ne doit pas être satisfaite
par falsification ; son remplacement est limité au prédicat de cette sélection.

Les champs parent_runtime_cleanup et copied_script_retirement gardent leur sens
actuel ; le script ne s'attribue pas les preuves du parent. Un exit0 interne ne
qualifie pas le retrait des copies Windows ou le runtime. Le parent conserve ses
verdicts propres et ne rend pas de succès si une obligation extérieure reste douteuse.

## Confidentialité, clôture et limite de connaissance

Même contrôle de confidentialité avant publication, captures bornées privées,
identités/jonctions et retraits sur succès, refus, signal et interruption. Les
sorties natives sont effacées comme auparavant, pas conservées « pour le prochain
parseur ». Aucun Stop/privilège supplémentaire, arrêt étranger, mutation du
stockage ou admission du conducteur privé. La proposition Windows après exit
reste distincte et Proposed ; elle ne bloque pas ce parent temporaire déjà borné.

Quel que soit le résultat, cette exception est épuisée après son appel. Encore
inconclusif => conserver cause/limite précise et instruire le repli Linux natif,
pas une succession de reconnaisseurs/reruns. Un compte0 ne prouve ni les dlopen
tardifs, ni le sandbox ou la cause Chrome. HTTP/Chromium restent INCONCLUSIVE.
Pas de nouvel OS, Linux natif/Windows/Kind mis en œuvre par cette décision.

Validation future ciblée : fixtures du parseur et du sélecteur, quota non réinitialisé,
unshare NOT_EXECUTED sans fausse preuve ancienne, compte positif connu malgré exit2,
sorties invalides/incomplètes, confidentialité et retraits. Aucune suite générale
redondante ni nouveau test exécuté par l'architecte. Pas de nouvelle ADR : détail
diagnostique réversible, sans dépendance structurante ou changement d'API publique.
