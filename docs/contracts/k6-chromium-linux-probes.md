# Sondes Linux ponctuelles — namespaces et dépendances Chromium

- Statut : **Accepted** ; Réalité : **Partial** — une sonde userns réussie,
  une commande ldd exécutée mais compte de dépendances inconnu ; aucun navigateur qualifié.
- Date : 2026-10-30 ; mission1 k6, hors Apparatus P0–P6.
- Base principale : `0f32e8dab504458074a353cb555139801db81487`, briefing
  `20261030-k6-userns-dependencies-research-followup.md`, rapport65 et ledger.
- Canon : [ADR0504](../adr/0504-validation-fonctionnelle-linux-k6.md),
  [V1, diagnostic parent](k6-functional-linux.md#addendum-diagnostic-parent-chromium--accepted-2026-10-30),
  [contrat causal](k6-chromium-causal-observation.md). Le candidat stockage149lignes
  reste gelé/non intégré ; il ne fait pas partie de cette proposition.

## Décision et frontière d'admission

Ratification ciblée du lead le2026-10-30, après reprise explicite de l'utilisateur
pour éprouver ses pistes namespaces/dépendances. Le candidat architecte a été
intégré byte-identique (SHA25600401fd3cb3b653cc8ba77dc171b8244a14a957dbf315fbc2ec5b3e10bdee7f0)
avant cette ratification. Le périmètre ci-dessous est admis ; les formules
propositionnelles de l'avis sont conservées comme provenance, pas comme une
autorisation plus large. Ni la collecte privée ni un nouvel essai navigateur ne
sont admis. Les corrections du conducteur restent sous revue indépendante.

**Compatible comme diagnostic ponctuel non qualifiant**, distinct de `browser`,
du direct Chromium et du relevé privé. La reprise utilisateur couvre cette enquête ;
le lead référence et ratifie ce seul écart avant sa consommation, sans nouveau GO
générique. Aucun changement Accepted n'est effectué par l'auteur de ce document.

Exception explicite à proposer : V1:425 exclut `unshare` de ses observations RO et
causal:530 l'écarte de la collecte. Ici seulement, autoriser la création éphémère
d'un **user namespace dans un enfant**, avec la commande fermée ci-dessous, sans
Chrome ni `PrivateSiteSession`. Ce n'est pas une lecture pure, ni une autorisation
de lancer Chrome sous `unshare`. Les autres exclusions demeurent intactes.

Un script ponctuel relu/testé sous `.tmp`, hors Git, suffit ; aucune modification
de `ops/tests/k6`, mode CLI permanent, copie de superviseur ou nouveau harness.
Pas de k6/Chrome, même pour `--version`, réseau, site privé, logs natifs, journal
hôte, trace/core, `sudo`, helper SUID, installation, changement sysctl/AppArmor/
seccomp/ACL/pin ou migration. Les fixes S-1/S-2 non vérifiés restent indépendants.
Le conducteur et son préflight restent interdits ; ces sondes ne certifient pas
son stockage et ne contournent aucune garde applicable ou refus natif.

## Admission d'une exécution future par le parent

1. Bail neuf de cette opération, inventaires avant/après, instance/nom→GUID/SID
   Windows et compte Linux non-root observés réellement ; pas d'ancien bail adopté,
   défaut WSL, autre distro ou manifeste pris comme preuve. Si une condition de
   sûreté applicable interdit le lancement, sonde **NON_EXECUTEE**, pas exception
   de stockage ni preuve d'une cause Chromium. Poursuivre les lectures permises.
2. Outils déjà présents, chemins absolus et provenance connus ; absence => état
   distinct, aucune installation. Pour l'ELF Chrome, revérifier archive/exécutable,
   hash du lock, identités et absence de substitution avec les contrôles existants
   `pinned_tools`/`verify_tools`, sans appeler les étapes de lancement/version.
   Un ELF ou interpréteur ELF non fiable interdit `ldd` : celui-ci peut solliciter
   le chargeur et n'est pas une simple lecture inerte de dépendances.
3. Répertoire privé du run hors Git, HOME ext4, chemins/modes/liens vérifiés ;
   sous-dossiers attendus par les gardes existantes, pas de sortie dans `/mnt/*`.
   Paramètres d'installation, captures et identités privées ne sont pas publiés.
   Préflight/runtime/retrait suivent les gardes et le cycle de vie déjà applicables,
   sans réutiliser les contrôles refusés du conducteur comme s'ils avaient réussi.

## Sondes et comparaison retenues

- **Contexte :** champs bornés ID/VERSION_ID de l'OS, version noyau/WSL, UID/GID,
  états Seccomp/Seccomp_filters/NoNewPrivs et identités de namespaces pertinents.
  Réutiliser les lectures précises déjà admises de userns/AppArmor ; aucun dump
  d'environnement, de `/proc` ou de journaux supplémentaire. Absent, interdit,
  illisible et zéro sont distincts. Les observations historiques ne sont pas fraîches.
- **Userns :** une commande fixe, non-root côté hôte :
  `/usr/bin/unshare --user --map-root-user -- /usr/bin/true`.
  Aucun autre namespace, montage, persistance ou programme libre. Le UID0 éventuel
  est interne au nouveau namespace, pas une élévation root sur l'hôte. Exit0 atteste
  seulement cette création/mapping/exécution dans ce contexte ; un exit non nul
  ne distingue pas à lui seul création refusée, mapping, exécution ou autre erreur.
  Ne pas le convertir en EPERM, « sandbox client coupable » ou cause Chrome.
- **Dépendances :** `/usr/bin/ldd` avec le seul chemin canonique de cet ELF vérifié,
  sans options de relocation, script fourni ou shell `-c`. Analyse locale privée
  sous la locale fermée existante. Export limité à état, exit réel et nombre de
  dépendances absentes : zéro seulement si exit0, sortie non vide, format dynamique
  reconnu non vacant et toutes les lignes interprétées sans ambiguïté. Code0 seul,
  sortie vide/tronquée/indécodable/inattendue ou échec => compte **null/inconnu**.
  Des absences peuvent être rapportées malgré exit0 ; aucun nom/chemin/adresse brut
  ne sort. Aucune conclusion sur les bibliothèques chargées ultérieurement par Chrome.
- **Comparaison :** distinguer A, invocation WSL explicite `--exec` non interactive,
  et B, enfant via les **mêmes** `Child.execute(browser=False)` et
  `child_environment(output)` de la source vérifiée. Mêmes instance, compte, ELF,
  argv de sonde et environnement fermé, sans héritage K6/proxy/LD_*. Pas un terminal
  utilisateur, ni émulation de toutes les conditions du navigateur.
  Recommandation minimale : relever le contexte du parent WSL puis celui observable
  dans la voie Child et y lancer les sondes. Cela compare des contextes, **pas** deux
  exécutions userns A/B si A n'a pas été mesurée. Ne pas ajouter un second superviseur
  pour combler ce manque : A non mesurée reste INCONCLUSIVE. Une comparaison effective
  des commandes A/B exige pour chacune les mêmes garanties de capture/identité/retrait.
  Déclarer les différences réelles de session/stdin/cwd/ascendance ; Child impose
  stdin DEVNULL, nouvelle session et subreaper parent, pas un sandbox ajouté par contrat.

Chaque commande au plus une fois par voie réellement mesurée, sans retries/fallback,
timeout10s puis cleanup existant. Garder une enveloppe parent qui laisse le cleanup
se terminer ; ne pas tuer WSL pour respecter artificiellement le délai d'une sonde.
Exception ciblée Accepted2026-10-30 : [une mesure ldd supplémentaire après
correction du lecteur](k6-ldd-parser-completion.md), sous quota global de deux
appels ldd pour l'enquête, sans second unshare ni nouvelle tentative navigateur.
Un enfant trop bref pour être identifié reste non observé : pas de délai artificiel
pour le retenir, ni de preuve d'identité/cleanup inventée à partir du seul exit.

## Confidentialité, retraits et oracles

Réutiliser les captures privées bornées de Child (64KiB par flux), son contrôle
de confidentialité et ses identités PID/UID/début de vie. Aucun stdout/stderr brut
dans le terminal enregistré, chat/MCP, exception, artefact ou commande journalisée.
Aucune catégorie textuelle libre de stderr : exit/signal/timeout/interruption,
observation complète/incomplète et résultat fermé seulement. Une violation de
confidentialité interdit publication et compte de dépendances, même sur chemin négatif.

Joindre les enfants/descendants possédés et l'invocation WSL par leurs identités,
vérifier absence de fuite et retrait des captures/script ponctuel privés après
analyse. Échec précoce, timeout et interruption gardent cette obligation. Aucun
arrêt par nom, kill forcé ou arrêt étranger ; une identité non observable ou un
retrait non prouvé reste INCONCLUSIVE/échec, jamais cleanup PASS par défaut.
Puis libérer le bail et arrêter seulement AIForAll devenue inutile ; préserver
instance/outils/données, pas de shutdown WSL global ni suppression de distro.

Résultats séparés : contexte, userns, ldd, confidentialité, cleanup. Même complets,
ils ne produisent aucun HTTP/Chromium PASS, résumé k6 ou preuve du sandbox Chromium.
AppArmor désactivé/seccomp0/limite userns positive ne prouvent pas une création ;
unshare réussi ne valide ni les autres namespaces ni la politique appliquée à Chrome.

## Vérifications avant exécution et limites

Photographie vérifiée après exécution le2026-10-30 : contexteUbuntu24.04/UID1000/
noyau6.6.87.2WSL2 ; userns exit0 ; ldd exit0 mais analyse incomplète/countnull,
script/parent exit2 DIAGNOSTIC_INCOMPLETE. Confidentialité/nettoyage/retrait Linux
PASS à leur portée, copies du parent retirées et runtime finalement observé arrêté.
Une commande de chaque type, pas de terminal utilisateur ou A/B de commandes.
[Mesures et limites](../../.opencode/missions/team_mvtz6p47_000q_4wmht0/reports/71-userns-reussit-dependances-encore-inconnues.md).
La première préparation avait été refusée avant sondes par l'observateur Windows
après exit ; sa limite et sa réconciliation ne sont pas réécrites en succès.

Revoir/tester le script et sa projection sur données connues : état absent/refusé,
exit non nul, ldd absent/missing avec exit0, sortie vide/invalide/partielle, sentinelle,
overflow, timeout/interruption, enfant non observable et cleanup manquant. Réutiliser
les tests existants de `test_runner.py` (environnement/confidentialité) et
`test_boundaries.py` (Child/limites/retraits), sans nouvelle infrastructure ni mock
HTTP. Leur présence n'est pas une preuve du script ou de ces sondes, non exécutés ici.

Pas de nouvelle ADR : expérience réversible sans nouvelle dépendance structurante,
API publique ou décision d'hôte. Le lead référence ce contrat depuis le canon lors
de son admission ; aucun contrat Accepted, index ou ledger édité dans ce delta.
Linux natif reste un repli à instruire séparément avec mêmes pins/gardes et preuves
propres. k6 Windows/Kind-Docker n'est ni cette cible Linux ni une migration admise.

Sources vérifiées pour cet avis : `lib/safety.py:253–260,319–548`
(`child_environment`, `Child`) et `run-linux.py:181–236` (contrôles de pin, sans
lancement), tests ciblés ci-dessus ; [unshare util-linux](https://raw.githubusercontent.com/util-linux/util-linux/master/sys-utils/unshare.1.adoc)
et [avertissement ldd](https://man7.org/linux/man-pages/man1/ldd.1.html), références
générales, pas versions des outils installés mesurées. Les sources Chromium156/
Puppeteer et le signalement d'un autre client sont vérifiés en parallèle par le
lead : aucun contenu Canonical expiré ni mesure étrangère adopté comme fait local.
