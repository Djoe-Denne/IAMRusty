# Revue indépendante — canal local de site privé

```yaml
reviewer: security
written_at: 2026-10-30T12:43Z
branch: master
head_sha: eb3d35530cb4d0b3e703c593b868c8047a512a6d
dirty: true
scope_slug: k6-private-site
verdict: GO_WITH_COMMENTS
security_decision: REVIEW_REQUIRED
related_paths:
  - ops/tests/k6/run-linux.py
  - ops/tests/k6/lib/private_site.py
  - ops/tests/k6/tests/test_private_site.py
  - ops/tests/k6/README.md
```

Rapport READONLY fourni par le reviewer et persisté par le lead avant correction.
Candidat `result_mvuysaq7_002h_3y9jqk`, session `ses_e732e1ec9ffefmvZm96gr2oByz`,
tâche `task_mvuykugk_002f_m27x96`. Aucun site réel fourni à cette revue.

## Verdict

Deux passes statiques effectuées sur le diff intégré `1c0036f4… → 968592f2…`,
identique au checkout examiné. Aucun chemin de fuite publique ou contournement CI
démontré. Un contrôle d'éligibilité à la divulgation privée accepte toutefois
certaines captures ambiguës : **S-1 nécessite revue avant admission de la collecte**.
Aucun CRITICAL/HIGH ; un MEDIUM. Les modifications étrangères de l'arbre n'ont pas
été examinées ni modifiées. Cette revue n'admet aucune collecte réelle.

## Finding S-1 — Un FATAL incomplet concurrent échappe au contrôle d'unicité

- Severity : MEDIUM ; Confidence : HIGH ; Category : other ; Scope : cross-file.
- Location : `ops/tests/k6/run-linux.py:420–431`, écriture appelée aux lignes516–518.
- Propriété violée : une capture ambiguë ou un enregistrement incomplet ne doit
  produire aucun relevé, selon la politique conditionnelle de divulgation privée.
- Surface : buffers stdout/stderr du diagnostic Chromium.
- Frontière : données natives non fiables → sélection autorisée pour le fichier privé.
- Capacité : influencer les octets natifs capturés ; session locale déjà admise.
  Une sortie native partielle ou concurrente inattendue suffit, sans attaquant réseau.

### Scénario

1. Un FATAL complet et admissible est présent dans un flux.
2. L'autre flux contient un second FATAL sans sa fin, sans overflow, secret ou UTF-8 invalide.
3. direct_text_observation retient la famille du premier ; le second n'est pas reconnu.
4. candidates ne compte que les correspondances complètes et conserve un seul élément.
5. Le premier couple est transmis à record malgré l'enregistrement concurrent incomplet.

### Preuve et impact

DIRECT_FATAL_FORMAT, ligne367, impose LF. Les lignes420–424 vérifient la famille
reconnue et le nombre de correspondances complètes, sans refuser un autre candidat
FATAL incomplet. Les tests refusent un incomplet isolé et deux complets, pas leur
combinaison. Contre-exemple établi statiquement ; **aucune PoC exécutée**.

Un relevé confidentiel peut être créé sans que sa condition de divulgation soit
établie. Impact limité au canal local désigné : aucune fuite publique ni élévation
de privilèges démontrée. Le contrat Accepted, lignes359–365 lors de la revue,
impose zéro relevé pour ambiguïté ou collecte incomplète.

### Correction recommandée et validation

Ajouter un veto conservateur propre au consommateur privé lorsqu'un autre candidat
FATAL est incomplet ou non conforme. Ne pas élargir les formats acceptés, modifier
le classificateur public, assembler les flux ou conserver leurs octets. Exiger un
seul candidat complet, sans candidat concurrent refusé.

Fixtures existantes : un complet et un second incomplet, même flux puis flux
distincts ; zéro relevé, exit non nul et absence de publication/contenu natif dans
les sorties. Étendre les négatifs de test_private_site.py:440–447 en conservant le
positif unique et les verdicts fonctionnels inchangés.

## Modèle de risque et passes

Actifs : couple source/ligne confidentiel et intégrité de son admission. Entrées :
sortie native hostile, malformée ou inattendue, opt-in et chemins locaux.
Abus : déclencher une écriture interdite ou faire sortir des données natives.
Mesures existantes : admission explicite, contrôles complets des flux et identités,
schéma fermé, permissions et sorties fermées. Mesure manquante : refus concurrent
incomplet décrit dans S-1.

### Passe défensive

- Opt-in et session entrée, marqueurs CI même vides refusés avant scrub/enfants.
- Refus nominal24 distinct du reçu22 ; callback parent séparé des fixtures.
- Deux buffers complets, UTF-8 strict, confidentialité, overflow sticky et identités enregistrées.
- Grammaire ASCII, six clés,1KiB ; aucune ouverture du chemin source.
- Descripteurs, O_NOFOLLOW/O_EXCL/O_CLOEXEC, modes/UID, device lié au HOME, hors Git.
- Couple absent des résultats publics, callbacks, repr et publishers.
- Retrait par identité, échéance UTC et plafond monotone, sans daemon ou effacement promis.

### Passe adversariale

- Préfixe valide insuffisant face à secret, UTF-8 invalide ou overflow tardif,
  même si cleanup devient prioritaire.
- Nettoyage d'environnement ne contourne pas le refus CI initial.
- Liens, collisions, permissions ouvertes et changements de device refusés sans adoption.
- Courses nécessitant processus hostile du même UID ou administrateur : pas de
  nouveau chemin démontré ici ; préflight exclut les workloads non fiables de cet UID.
- Chaîne « complet + concurrent incomplet → unicité apparente → écriture » ouverte : S-1.

### Supply chain et CI

Aucune dépendance externe nouvelle. Workflow, Child, pins et scénarios inchangés
dans le diff vérifié. Le workflow ne transporte pas l'opt-in privé et conserve
les publications explicitement allowlistées.

## Limites et conditions distinctes

Pas de réaudit global des services, auth ou dépendances Rust. Ne pas traiter les
callbacks de fixtures comme preuves du parent réel, réattribuer des résultats
Linux historiques au delta ou collecter un site inconnu pour reproduire S-1.
Pins, arguments Chromium, sandbox, scénarios, exits et projections publiques à conserver.
Aucun nouveau reconnaisseur positif, logger, artefact natif, runtime ou infrastructure.

Même après résolution, la collecte attend les preuves réelles du parent : identité
hôte/HOME, FS/ACL, bail, provenance/corrélation, créneau de lecture et disponibilité
jusqu'au retrait vérifié. Un simple callback True n'apporte pas ces preuves.
Leur absence actuelle n'est ni une vulnérabilité supplémentaire du module ni une admission.

## Vérifications réellement exécutées

- git diff --check normal et sur diff intégré : exit0.
- Scope intégré comparé au checkout : exit0, aucun delta ; quatre fichiers confirmés.
- Aucune écriture ni ressource créée/redémarrée par le reviewer.

Non exécutés : unittest, k6/version Chromium, inspections, validate/run/browser/selftest.
La preuve Windows du lead162/exit0/skips8 est rapportée, pas réexécutée et pas
une preuve Linux. Les résultats de la vérification Linux parallèle n'ont pas été
intégrés à cette revue. HTTP, Chromium et cleanup parent ne sont pas qualifiés.

Revue liée à eb3d355, outillage968592f ; revérifier lignes et diff après correction.
