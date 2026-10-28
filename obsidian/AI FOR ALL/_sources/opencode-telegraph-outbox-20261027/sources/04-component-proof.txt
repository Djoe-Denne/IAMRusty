# 132. La reprise après expiration du verrou est prouvée sur le composant

## Résultat

Après le nettoyage vérifié, l'unique test ciblé a réussi : **1 PASS, 0 échec,
0 ignoré, 6 filtrés**, Cargo0, durée libtest41,02s. Aucun changement de production,
de lease ou d'oracle n'a été nécessaire.

Le test crée une vraie réservation SQL, annule son propriétaire local, puis
reconstruit un dispatcher sur la même base. Il observe15 cycles avant expiration
sans reprise anticipée, puis1 cycle après expiration et1 nouvelle prise en charge.
Le même événement est réellement envoyé sur SQS, reçu et corrélé aux données SQL.
Les comptes métier sont inchangés ; les anciens et nouveaux parents sont détruits
et joints. Cela valide la reprise contrôlée du composant, pas toute l'application.

## Bornes réellement observées

L'ancien verrou expire à15:22:18,510192Z. Le nouveau claim SQL est daté de
15:22:18,696808Z, soit environ187ms après cette éligibilité. Le transport retourne
un succès, puis les observations finissent30,244094s après l'entrée de
reconstruction, avant l'échéance fixée une seule fois à54,989538s. Le bilan
atteste16 cycles commencés et terminés,1 claim,1 succès du transport et1 message.
La phase reconstruction et cleanup mesurée vaut environ30,244s ; pas de reset
de deadline ni lecture tardive pour sauver le test.

Ce protocole est une borne de test, pas un engagement produit de livraison
exactly-once ou une garantie contre toute panne distante.

## Ressources et contrôle frais

Le lead a recoupé les résultats du lanceur avec un nouvel inventaire Docker :
4 créations attestées,3 désormais absentes,0 possédée active,0 nouvelle identité
inconnue. La fixture SMTP restante est arrêtée avec exit2 et conservée ASK pour
diagnostic. Le lanceur l'avait classée génériquement yes : le registre a été
corrigé par une ligne append-only ASK, sans suppression. Deux volumes opaques
restent également ASK. Les14 conteneurs préexistants conservent identités et états,
ainsi que leurs136 volumes ;138 volumes sont présents après campagne.

Aucun Cargo ne reste actif. Desktop/WSL ne sont pas arrêtés : le workload étranger
reste actif et n'est pas possédé. RAM non mesurée. Aucun prune, arrêt global ou
suppression de volume. Recoupement fait par le lead, pas par un nouveau reviewer.

## Portée et suite

La preuve auparavant absente après expiration est désormais acquise pour ce
scénario contrôlé. La cause historique du timeout109 reste inconnue ; le rouge
109 et le défaut de lancement122 restent archivés et ne sont pas réécrits.
Le PASS ne démontre ni toute la course SQL annulée de l'application, ni droits
OpenFGA/replay/HTTP de la verticale finale. Celle-ci, la publication du SDK et le
pin final restent bloqués et ne sont pas lancés par ce résultat. Aucun nouveau
test ou rerun automatique n'est nécessaire pour clôturer cette tranche ciblée.

## Preuves et commande

Commande réellement exécutée :
`cargo test --locked -p telegraph-service --test outbox_runtime_tests committed_claim_recovers_same_event_after_measured_expiry_with_fixed_rebuild_deadline -- --exact --test-threads=1`.

Source inchangée : AI1f2dd30, SDK locald60, testSHA445f729a ; lanceurSHAa77dba1d.
Archives privées sous TI `.tmp/post-expiry122-6412a9c40ae64bf4/` : log,
summary.json, inventaires avant/après, ledger fixtures, bail,
`lead-cleanup-verification.json`. Registre contemporain
`.tmp/docker-resources-20261027.log`, append-only.

Localisation : preuve composant Telegraph outbox. Impact : reprise après lease
vérifiée sans modification produit. Limites : pas de quitus global, publication,
pin, nouvelle revue ou résultat Sonar ; dettes Clippy antérieures inchangées.
