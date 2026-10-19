---
name: aiforall-test-evidence
description: Prépare et archive des campagnes Apparatus, IT Windows et E2E Kind avec preuves H1-H4, registre Docker contemporain et préflight des remplacements. À charger avant un run réel, une capture de preuves ou la revue finale d'une campagne de tests AIForAll.
---

# Campagnes de preuves AIForAll — pièges évitables

Canon : `AGENTS.md`, règles infra-safety/docker-resource-registry/agent-handoff,
skill `local-runtime-lifecycle`. Ce skill ne donne aucune permission de runtime,
suppression, nouveau test ou contournement de hook.

## 1. Préflight avant tout Cargo ou runtime

- Fixer HEAD, sources exécutées, nombre exact de tests, features, GO et unique bail Cargo Windows ; target E: effectivement vérifié. Ne pas confondre compilation/listing/oracle scripté et preuve live.
- Lire **les effets transitifs** du scénario : `ensure` peut être non destructif alors que le corps E2E ou le reconcile supprime Pod/AdmissionRecord ou un helper remplace son prédécesseur.
- Capturer les UID/IDs avant. Nom/digest/image n'établissent jamais propriété. Le cycle m6 local est accepté par décision utilisateur du 18 octobre 2026 ; cela ne permet ni de prétendre une préservation intégrale ni de viser production/autre projet/contexte protégé. Pas de durcissement de production glissé dans un lot test/documentation.
- Si le contexte partagé échoue, ne pas conclure clusterdown : exporter vers **un kubeconfig privé dédié** et vérifier la cible effective. Ne jamais modifier le kubeconfig partagé ni imprimer client-key-data/certificates.

## 2. Formats et outils Windows

- Logs PowerShell possibles en UTF-16/BOM ; registres anciens en UTF-8, CP1252 ou mélange **par ligne**. Lire octets, décoder strictement avec provenance d'encodage ; tester le parseur avant lancement. Pas de `errors=ignore/replace`, troncature, réécriture ou « correction » de l'historique.
- Pour le texte analysé seulement, enlever les NUL issus d'un décodage inadéquat n'autorise pas à modifier l'archive source ; préférer décodage correct. Une recherche ASCII vide ne prouve pas absence d'erreur.
- Context Mode exécute ici du bash : chemins Windows d'exécutables explicites ; éviter `$variable` PowerShell interpolé par bash. Préférer un script Python court créé par patch. Pour les scripts JS transmis, `String.fromCharCode(10)` évite le piège de newline littéral ; corriger une erreur de script dans le même outil, ne pas diagnostiquer une panne MCP.
- PATH du sandbox ≠ PATH Windows. Tester l'exécutable réel avant de déclarer CLI absente ; ne pas installer/reconfigurer globalement pour un simple chemin.

## 3. Création et cleanup traçables

- Enregistrer **à la création**, append-only avec flush, vrais IDs retournés ou observés par Engine, provenance explicite. Si un témoin d'événements est nécessaire, curseur basé sur l'heure **du daemon**, pas l'horloge Windows/WSL ; disponibilité vérifiée avant création et drain vérifié en fin de phase.
- Distinguer témoin indépendant et ID retourné par la bibliothèque : ne pas inventer ce dernier. Création opaque = UNKNOWN+ask. Prédécesseur remplacé : conserver son ID et état réel, pas un faux « absent ».
- Stop gracieux par ID des seules ressources possédées sans bail actif ; caches/volumes/objets préexistants non adoptés, pas prune/forçage. Comparer avant/après et rapporter les ressources ask/no conservées.
- Processus natif de test ≠ worker Ensemble. Après dispatch background, attendre sa notification sans polling. Rejet au préflight = zéro test exécuté, journaliser cause et tentative ; ne pas effacer le dossier vide pour masquer la chronologie.

## 4. Captures discriminantes

- Une erreur quelconque de wget ne prouve pas le réseau. Même flux réel/scripté, Pod API indépendant d'Endpoints, UID/IP/Ready/nonDeletion vérifiés aux phases définies ; positif préalable ; négatif GNU/exit/diagnostic/durée discriminant. Ne pas affirmer causalité exclusive Netpol ou dataplane DNS/Service non testé.
- Libtest peut joindre `test ... ... ` et `M6 H1 ...` sur **la même ligne**. Rechercher le marqueur, exiger une occurrence, puis fullmatch du schéma fermé ; pas `^M6 H1` seul. Vérifier liens catalogue/enveloppe/CRI, hostname/hops, counts exacts et zéro skip/filtered dans log brut.
- Assertions API : messages fermés sans `Debug` des opérandes ; tester égalité, échec et absence de deux sentinelles. Un test helper ne prouve pas automatiquement le câblage des callsites : le vérifier séparément ou nommer le gap.
- Archive allow-list explicite + tailles/SHA256 et source HEAD. Jamais zip récursif d'un dossier ayant contenu kubeconfig/credentials. Pas d'assainissement silencieux : copie transformée distincte, empreinte source conservée, périmètre de scan annoncé.

## 5. Reviews et livraisons

- Fermer les findings de preuve/confidentialité avant captures. Verdict fonctionnel, état infra et conformité de portée restent distincts. Une acceptation humaine ultérieure est datée, ne réécrit ni les faits ni le GO initial.
- Hook Windows `os error 206` sur long worktree : préserver le diff, livrer un patch borné, vérifier les hashes des fichiers puis commit canonique au hook normal si approuvé. **Jamais** rejouer une exception `--no-verify` déjà consommée ni raccourcir globalement config/règles sans GO.
- Diff identique : réutiliser les preuves valides, pas de rerun redondant. Clippy strict rouge sur dette prod différée reste rouge ; succès des oracles ne la résout pas.

## Sources et limite de réutilisation

Retour vérifié : `docs/missions/20261018-apparatus-m6-proof-hardening-results.md`,
ledger `.cursor/handoffs/20261018-m6-proof-hardening-preparation.md`, briefings associés.
Ces observations n'affirment pas qu'un script temporaire soit une fixture officielle.
Revalider l'environnement et le protocole ; ne pas lancer automatiquement les scripts
`.tmp` d'une ancienne campagne ni reproduire ses suppressions sur une nouvelle cible.
