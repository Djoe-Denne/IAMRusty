# Revue indépendante bornée — diagnostic Chromium direct

## Mandat READONLY

Diff `14d55e9813996a737b92b7e5aff2d20536ede47c..e9af3b673319c62577b82f612f868846ae4dfd79`, cinq fichiers : run-linux.py, nouveau test_direct_chromium.py, workflow k6 dédié, README et IMPLEMENTATION. Pas de source edit/runtime/CI/Cargo/provisionnement. Deux passes ciblées, pas réaudit global des pins ou anciens diagnostics.

Risques distincts à vérifier : exécution directe d'un binaire natif par le parent (pins/identités avant Child, argv/profil/environnement fixes), confinement/about:blank/traﬁc d'arrière-plan, lecture strictement bornée des seuls fichiers hôte admis, sorties fermées y compris sur privacy/cleanup failure, non-qualification systématique et absence d'artefact d'échec publié. CI : condition d'appel séparé après échec browser et succès outils, même job/UID/racine/pins, pas de fallback dans browser ou masquage d'exit.

## Canon et faits vérifiés

Addendum diagnostic parent Chromium **Accepted 2026-10-30**, docs/contracts/k6-functional-linux.md, intégré14d55e9 avant code ; ADR0504/V1 Accepted/Partial inchangés. Diagnostic compatible mission1 mais ne remplace pas k6/browser ; aucun résultat natif ne qualifie HTTP/Chromium, aucune modification de politique, privilège, wrapper, paquet ou déploiement autorisée.

L'utilisateur a autorisé les essais supplémentaires avec recherches/réflexion entre chaque essai. La recherche épinglée et les choix d'arguments sont documentés. Le motif est que le parseur k6 peut perdre un FATAL Chromium ; la cause réelle n'est pas encore établie. Aucun nouvel essai direct n'a été exécuté à ce stade.

Lead : cinq fichiers récupérés par patches et strictement identiques au candidat sur disque. Tests Windows natifs92, exit0,7skipsPOSIX (85 exercés), commande C:/Python313/python.exe -B -m unittest discover -s ops/tests/k6/tests -p test_*.py, temp limité au dossier hôte approuvé. Diff staged check et vrai hook cargo fmt réussis lors du commite9af3b6. Aucun nouveau résultat Linux/live ni WIP étranger inclus.

## Oracles de revue

- Le mode diagnose-chromium doit toujours sortir nonzero avec HTTP/Chromium INCONCLUSIVE, même exit natif zéro et témoin observé ; aucune chute vers le parcours HTTP ou publish.
- Les erreurs privacy/cleanup restent bloquantes et ne sont pas neutralisées par un diagnostic utile. Une catégorie lexicale fermée connue peut rester visible lors d'un refus de flux, mais aucun texte/URL/match ne doit passer ; sentinelles/invalides/overflow doivent être retenus.
- Pas de preuve cleanup sans identité contemporaine possédée. Même Child/sandbox_witness et pas de kill forcé/arrêt étranger. Pas de nouveau scanner, journaux hôte ou sonde unshare.
- Le résultat de host_metadata est revalidé, états absents/refusés/illisibles/zéro distincts. Les réglages ne prouvent pas une cause AppArmor.
- Le même job contient le cas k6 et au plus un direct ; l'échec initial reste rouge, upload succès-only inchangé. Aucune permission/secret/événement privilégié supplémentaire.

## Retour

Findings démontrés avec fichier/ligne/scénario et limites, suggestions séparées, pas conclusions d'auteur comme preuve. Les tests injectés ne sont pas qualification Chromium. Le lead conserve source gelée, correction même auteur et publication/essai après revue. Briefing résultat au chemin dédié si le profil le permet ; aucun ledger partagé modifié.
