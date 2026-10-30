# Revue indépendante fresh du conducteur privé Windows/Linux

## Périmètre intégré

Source `195059a36007352c3560e75757ee8f16cc4d4d3f`, trois nouveaux fichiers :
`ops/tests/k6/local-private-parent.py`, `local-private-parent.ps1`,
`tests/test_local_private_parent.py`. Base source `cbab06e` : les 22 fichiers
existants sont byte-identiques, dont consommateur, runner/Child, pins, scénarios et
workflow. Revoir le code intégré et son diff réel, pas l'ancien arbre auteur b316.

Contrat causal ACTUEL, amendement pipes Accepted avant code dans `b6d32b3` ;
ADR0504/V1 Accepted/Partial. Lecture du site privée par l'utilisateur uniquement,
aucun contenu inconnu dans les outils/chat/GitHub. Ce conducteur compose les APIs
existantes, pas nouvelle autorité, service, collecteur ou campagne.

## Faits et preuve disponible

Le lead a lu les trois fichiers complets, récupéré le candidat par patch puis
vérifié l'égalité octet pour octet. Les 22 sources gelées sont inchangées.
Windows : `C:/Python313/python.exe -B -m unittest discover -s ops/tests/k6/tests
-p test_*.py`, temp hôte approuvé : **189 tests/5.034s/exit0/skips8**, 181 exercés,
dont les 11 fonctions de test PowerShell natif du nouveau fichier. Parseur PS :
0 erreur/exit0. UTF-8 strict, pas de caractères de remplacement/blancs finaux ;
diff-check normal et staged0, commit au hook normal. Pas de conclusions auteur
fournies comme preuve de sécurité.

Le nouveau conducteur n'a pas été exécuté contre WSL. Profils privés et bail de
préflight encore à préparer ; aucun rendez-vous ou site réel collecté. Les 165
tests Linux antérieurs qualifient seulement le consommateur gelé, pas ce delta.
Le lead pourra vérifier en parallèle les nouvelles fixtures sous Linux ; pas
d'armement ou préflight réel des observateurs avant la revue applicable.

## Questions bornées de revue

- Activation : CI avant processus, mode préflight réellement non armant, capture
  réservée à la console locale prête ; entrée redirigée, fichier déclaratif ou
  simple absence de CI ne vaut pas permission de collecte.
- Observations : Lxss/SID/identités/ACL/configuration/bail Windows réels et
  renouvelés, invocation GUID fixe et compte non-root ; HOME ouvert/ext4 et
  provenance/refus historique/outils Linux liés aux faits réellement vérifiés.
  Examiner noms/parents/liens/substitutions et refus sur changement, sans supposer
  le profil attendu fiable à lui seul ou les contrôles du parent déjà disponibles.
- Protocole : canaux privés du processus créé, schéma fermé et bornes 4 KiB,
  framing, ordre, IDs/deadline/HOME et budget monotone 30s de bout en bout local.
  EOF, doubles/tronqués/surdimensionnés, stderr, retards et faux acquittements.
- Confidentialité : aucun site/corps natif, erreur libre, chemin de relevé ou
  payload rejeté dans frames/stdout/stderr/journaux/publishers. Le parent ne lit
  jamais site.json, même comme profil ou preuve ; sentinelles et négatifs pertinents.
- Lecture/retrait : état READY, LU courant depuis la vraie console, échéances
  UTC/monotone, contexte vivant jusqu'au retrait et absence vérifiée. Interruption,
  perte du contrôleur/bail, retrait impossible et libérations avant verdict fermé.
- Processus : handle natif retenu, PID/création/image/SID et fin, drainage borné
  sans deadlock, pas d'interop, kill forcé, adoption par nom ou runtime étranger.
  La fin WSL ne prouve pas le cleanup Linux ; aucune clôture verte par agrégation
  partielle ou par un code diagnostique non nul attendu.
- Oracles : les tests exercent-ils ces contrats observables, pas seulement helpers
  ou comparateurs simulés ? Vérifier les valeurs réellement transmises, négatifs
  et gaps limités aux risques modifiés, sans imposer un nouveau framework.

## Modalités

Fresh READONLY, deux passes sécurité défensive/adversariale. Retour findings
bloquants séparés des suggestions et limites hors périmètre, fichier/lignes,
contre-exemple et correction attendue. CRITICAL/HIGH bloquent ; défaut prouvé
d'une condition obligatoire de collecte reste à résoudre avant activation.
Pas d'audit global, série de rôles QA ou nouveau harness demandé par défaut.

Aucun runtime, WSL, Chrome/k6, Cargo, CI, installation, lecture d'un site réel,
écriture de production/ledger ou agent enfant par reviewer. Des tests purs
bornés sur fixtures connues sont possibles si utiles et permis, sans lancer le
conducteur contre l'hôte. Le lead persiste le rapport avant toute correction.
Un PASS statique ne vaut ni preuve Linux/native, admission de collecte, ni
qualification HTTP/Chromium/cleanup parent.
