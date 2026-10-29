# Review briefing

```yaml
reviewer: security
written_at: 2026-10-29T21:40Z
branch: master
head_sha: db352be1ebe01eb275fd74d8cdee6f7298b99bf1
dirty: true
scope_slug: k6-private-directory
verdict: BLOCK
related_paths:
  - ops/tests/k6/lib/safety.py
  - ops/tests/k6/run-linux.py
  - ops/tests/k6/tests/test_boundaries.py
  - ops/tests/k6/IMPLEMENTATION.md
```

Source : reviewer indépendant pool-5, session ses_e767aa95effe5s3xLdh1pnBgQs, candidat result_mvu2j7hb_001o_58pild/message msg_1899b5b49001Nw6ZwJ9wVUhtYG. Rapport reçu et persisté par le lead avant reprise auteur. Aucun correctif appliqué par le reviewer.

## Verdict

BLOCK : S-1 HIGH, substitution locale d'un chemin d'exécutable après
vérification de son empreinte, rendue possible par un répertoire ancêtre
contrôlable par un autre UID nouvellement admis.

Revue limitée au delta db16aa7..db352be et aux consommateurs directement
atteints. Aucun runtime, provisionnement, Cargo, correctif ou test exécuté.

## Finding

ID: S-1
Severity: HIGH
Confidence: HIGH
Category: race
Scope: cross-file

Location: ops/tests/k6/lib/safety.py:79–94
Consumers: ops/tests/k6/run-linux.py:123,130–149

Security property violated:
Intégrité du binaire vérifié et séparation entre UID locaux.

Attack surface:
K6_TOOLS_DIR traité par pinned_tools(), également utilisé par les modes
tools/browser sans cible applicative.

Trust boundary:
Utilisateur local pouvant modifier un ancêtre du répertoire d'outils →
exécution d'un binaire sous l'UID de l'opérateur.

Attacker capability:
Un autre UID peut modifier les entrées d'un ancêtre accessible :
par exemple un répertoire partagé inscriptible non-sticky, ou un ancêtre
appartenant à cet attaquant. Il ne peut initialement ni lire ni modifier
le répertoire d'outils0700 appartenant à l'opérateur.

Exploit scenario:
1. L'opérateur fournit un répertoire d'outils0700 sous un tel ancêtre.
   Les fichiers officiels et leurs modes sont initialement conformes.
2. private_path(directory=True) accepte désormais le répertoire sans
   vérifier que ses entrées ancestrales sont protégées contre remplacement.
   Le cas direct sous parent public/étranger était refusé avant ce delta.
3. Pendant le dernier calcul d'empreinte, après ouverture du fichier,
   l'attaquant renomme le répertoire d'outils et remplace son ancien nom
   par un lien vers une arborescence qu'il contrôle. Le renommage d'une
   entrée ne nécessite pas de traverser le répertoire0700 déplacé.
4. Le descripteur déjà ouvert continue à fournir les octets officiels :
   digest(binary) peut donc réussir.
5. version_checks() rouvre ensuite le chemin stocké, sans revalidation
   d'identité. Child.execute() lance le remplacement sous l'UID de
   l'opérateur, avant tout contrôle de sa sortie ou témoin sandbox.

Ce scénario nécessite un chemin sous ancêtre attaquable et un attaquant
local concurrent. Il ne concerne pas le cas normal d'un /tmp root-owned
sticky correctement utilisé ni une chaîne ancestrale protégée.
Aucune exploitation de l'environnement actuel n'est affirmée.

Evidence:
- safety.py:84–88 ne contrôle maintenant que l'objet courant.
- safety.py:89–93 applique la même validation au parent d'un fichier,
  mais n'établit pas la protection des entrées ancestrales.
- run-linux.py:123 appelle cette garde pour K6_TOOLS_DIR.
- run-linux.py:134–138 vérifie les octets puis conserve un Path.
- run-linux.py:148–149 exécute ensuite ce chemin.
- La vérification canonique est antérieure au remplacement ; elle ne
  protège pas les résolutions ultérieures.
- Le témoin sandbox Chromium ne protège pas ces appels --version :
  ils sont lancés comme enfants natifs ordinaires.

Impact:
Exécution de code choisi par un UID local moins privilégié avec les droits
de l'opérateur ; accès possible aux fichiers de cet utilisateur.
La même absence de stabilité peut affecter les chemins de sortie.
Le verrou HTTP n'atténue pas ce chemin tools/browser.

Affected components:
Garde private_path et lanceur k6 Linux, notamment pinned_tools/version_checks.

Recommended fix:
Ne pas rétablir l'exigence excessive « tous les ancêtres0700 et même UID ».
Distinguer :
- confidentialité de l'objet et de son conteneur immédiat ;
- protection de la chaîne de noms contre remplacement par un autre UID.

Avant d'admettre un répertoire d'outils ou de sortie, refuser les ancêtres
contrôlables par un UID non fiable : propriétaire non fiable, ou écriture
groupe/autres permettant le remplacement sans protection sticky adéquate.
Autoriser les ancêtres système sûrs0755 et les cas sticky correctement
possédés/protégés. Contrôler toute la chaîne pertinente, pas seulement
l'absence initiale de lien.

Une alternative fondée sur des handles doit couvrir aussi les résolutions
ultérieures par les enfants ; protéger seulement mkdir/open côté Python
ne suffirait pas.

How to validate the fix:
Dans des fixtures temporaires uniquement :
- accepter ancêtres système/publics non inscriptibles0755 ;
- accepter le cas sticky attendu avec les propriétés correctes ;
- refuser un ancêtre accessible non-sticky inscriptible par un autre UID ;
- refuser un ancêtre appartenant à un UID non fiable ;
- vérifier le refus avant création de sortie ou appel de Child.execute.

Regression test:
Ajouter des oracles de propriété/modes pour ces chaînes et un scénario
déterministe de substitution de chemin entre vérification et utilisation,
sans exécuter de binaire hostile. Réutiliser les fixtures existantes ;
aucun serveur ni nouveau runtime nécessaire.

CWE: CWE-367

## PASS A — Défensif

Les contrôles ponctuels restent présents :
- chemin absolu et canonique ;
- propriétaire et droits de l'objet ;
- type répertoire/fichier, fichier privé non inscriptible ;
- contrôle du parent immédiat pour les fichiers ;
- exclusion Git ;
- création exclusive d'une sortie nouvelle.

Le relâchement supprime toutefois aussi une protection de stabilité des
entrées de répertoire. Un répertoire0700 n'empêche pas son renommage par
une personne autorisée à modifier son parent.

Les négatifs sentinelle/overflow exigent maintenant les codes exacts :
PRIVACY_FAILED et OUTPUT_LIMIT. C'est discriminant contre l'ancien
échec prématuré PATH_INVALID.

## PASS B — Adversarial

Chaîne nouvelle :
ancêtre modifiable par un autre UID → renommage/substitution →
empreinte calculée sur l'ancien descripteur → exécution via le nouveau chemin.

Cette chaîne contourne les protections ponctuelles sans casser SHA256
ni modifier les fichiers officiels initialement contrôlés.
Elle reste atteignable alors que HTTP est volontairement fermé.

## Mini threat model

ASSET: intégrité des outils et droits de l'opérateur.
ATTACKER: UID local distinct.
ENTRY POINT: répertoire d'outils admis sous chaîne ancestrale attaquable.
TRUST BOUNDARY: contrôle des noms de fichiers → exécution sous un autre UID.
CAPABILITY: renommer/remplacer des entrées sans accéder au contenu0700.
ABUSE CASE: faire exécuter un autre fichier après vérification du pin.
EXISTING MITIGATION: canonicité, modes, propriétaire, empreintes.
MISSING MITIGATION: stabilité/protection de la chaîne de noms jusqu'à l'usage.

## Tests et preuves

Effectués par le reviewer :
- diff exact des trois fichiers examiné ;
- consommateurs private_path/pinned_tools/version_checks recoupés ;
- HEAD confirmé db352be, sans delta local sur les trois fichiers ;
- git diff --check : exit0, avertissement CRLF sur mémoire Serena étrangère ;
- diff-check db16aa7..db352be : exit0.

Preuve du lead réutilisée, sans relancer :
- log .tmp/k6-linux-unittest-1793309664423.log lu :
  « Ran40tests », « OK » ;
- exit0/aucun skip sous Ubuntu24.04/WSL non-root rapportés par le lead.

Les tests verts ne couvrent pas la substitution concurrente décrite.
Le nouveau test public0777 est placé sous une racine0700 de fixture :
il ne démontre pas la résistance face à un autre UID pouvant réellement
atteindre et modifier cet ancêtre.

Non exécutés par cette revue :
- PoC multi-UID ou substitution instrumentée ;
- unittest Linux ;
- k6/Chromium version, inspect, validate, run, browser, selftest ;
- runtime, provisionnement et Cargo.

## Do not redo

- Ne pas refaire l'audit CI, cible HTTP, pins ou sandbox.
- Ne pas confondre public0755, sticky1777 protégé et inscriptible0777
  non-sticky : ils n'offrent pas la même protection contre renommage.
- Ne pas considérer chmod0700 sur l'enfant comme protection de son nom.

## Anti-goals

Aucun changement d'autorité cluster, cible HTTP, TLS/sandbox ou workflow.
Aucun rétablissement aveugle de la règle trop stricte sur le grand-parent.
Aucun runtime ou payload hostile nécessaire pour le correctif.

## Open questions

Aucune question d'architecture nécessaire pour corriger cette garde locale.
Correction à reprendre dans la session auteur existante.
Après correction HIGH, nouvelle revue indépendante du delta obligatoire.

## Staleness

Lié au HEAD db352be. Les WIP étrangers sont hors périmètre et préservés.
Aucun fichier, branche, worktree ou ressource créé par le reviewer.
