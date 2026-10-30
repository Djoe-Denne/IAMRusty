# Fermer S-1 uniquement — veto privé et régression intégrés

## Source et périmètre

Même session reviewer18, suite de la revue persistée
`20261030T1243Z-security-k6-private-site-eb3d355.md`. Pas de réaudit général.
Base `968592f2efec114d6f7c0c1c52c84fb07b505cab`, correctif intégré
`cbab06eb68079f43ddb2246fc69c7dbdde68f071` ; deux fichiers uniquement :
run-linux.py (+5 lignes) et tests/test_private_site.py (+64/-6).

Examiner le vrai diff et les oracles, sans utiliser les conclusions de l'auteur
comme preuve. Toutes les fonctions du lanceur sauf `_private_site_candidate`
sont AST-identiques à968 selon vérification du lead. Les20autres fichiers de
base sont byte-identiques, dont module privé, Child, workflow, pins et scénarios.

## Question de fermeture

Le veto privé compte les occurrences littérales de FATAL après la vérification
complète des deux flux ; toute occurrence supplémentaire refuse le relevé,
y compris un candidat tronqué/non conforme. Ce choix est conservateur : une
mention ordinaire peut aussi empêcher un relevé. Aucun format positif ou
classificateur public n'est élargi et les flux ne sont pas assemblés.

Vérifier si le contre-exemple S-1 est réellement fermé dans le même flux et entre
flux opposés, avec ordres inversés, sans appel record/fichier/publication ou
contenu natif dans les sorties, tout en conservant le positif unique. Signaler
seulement une régression concrète introduite par ce delta si nécessaire.

## Preuves et limites

- Lead Windows :165tests/1.130s/exit0/skips8,157exercés, suite complète.
- Diff-check normal et staged exit0 ; commit au hook normal.
- Auteur rapporte ROUGE3tests/32sous-cas en échec avant code, GREEN3tests/
  38sous-cas ; ceci est attribué, pas une PoC réelle ou conclusion de sécurité.
- Les162tests Linux précédents concernent968, pas ce correctif. Le lead vérifiera
  les165tests de fixtures connues en parallèle ; aucune capture réelle n'est admise.
- Le contrôle parent réel et le rendez-vous de lecture restent des conditions
  opérationnelles distinctes. L'architecte précise leur procédure sans modifier
  ce delta. Aucune source native inconnue n'est fournie au reviewer.

Retour READONLY limité : S-1 fermé ou encore ouvert avec preuve, tests pertinents
et éventuels gaps bornés. Pas de WSL, Chrome/k6, CI, Cargo, écriture, collecte ou
nouveau worker. Le lead persiste la fermeture ; aucune admission de collecte ou
qualification HTTP/Chromium/cleanup parent ne découle de ce seul retour.
