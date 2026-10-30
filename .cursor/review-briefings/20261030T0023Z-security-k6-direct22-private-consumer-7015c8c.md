# Review briefing — consommateur privé direct22

```yaml
reviewer: security
written_at: 2026-10-30T00:23Z
branch: master
head_sha: 7015c8c3dfd8053dcc74fd9f6e0dd7a2fe9ddff8
dirty: true
scope_slug: k6-direct22-private-consumer
verdict: PASS
source_range: d47b80508f71cfffbbd5d6caf2146f949997a954..0601f6f73a93a2fedb9b8717450d06acd6a5fa67
```

## Verdict

PASS ciblé après passes défensive et adversariale. Aucun défaut de sécurité démontré dans le consommateur privé ajouté. Aucun PASS global, Linux ou Chromium n'est déduit de cette revue statique.

## Findings

None.

## Périmètre et faits vérifiés

Sept fichiers runner/README/workflow et tests direct/comparaison/provisionnement/nouveau test_direct_followup.py présents, identiques à0601f6f. Préfixe job24 identique à la base ; libs, pins, scénarios et provisionneur inchangés.

Modèle de menace borné : entrée locale hostile, autre UID ou sorties enfant inattendues ; frontières fichier privé→admission, flux enfant→projection et PR→runner. Risques examinés : lancement répété/non admis, fuite et faux PASS. Aucune vulnérabilité démontrée dans ce delta.

## Passe défensive

- run-linux.py:590–607 : lecture privée bornée/JSON strict, chemin canonique, parent commun, métadonnées et état admissible avant tout enfant de version ou navigateur ; marqueur créé exclusivement avant lancement.
- safety.py:193–216, inchangé et consulté à cette frontière : propriétaire courant, fichier régulier readonly, parent privé, hors Git et chaîne canonique contrôlée.
- run-linux.py:556–587 : schéma fermé, timestamps/durée bornés120s, UID/environnement/versions/hashes et observations hôte corrélés ; résumé valide, cas incomplet, outils/privacy/cleanup PASS requis.
- run-linux.py:469–502 et827–845 : signal privé seulement après recontrôle des flux complets et du résumé courant ; overflow, confidentialité défaillante ou cleanup non PASS ne l'admettent pas.
- Workflow:119–129 : étape distincte après échec réel de comparaison. Aucune permission, action, publication d'échec ou installation ajoutée.

## Passe adversariale

- Entrées absentes, tronquées, hors schéma, périmées ou de contexte différent non admises ; pas de projection de valeurs arbitraires.
- Symlink/substitution par un autre UID soumis aux gardes privées existantes. Marqueur xb refusant préexistence et double consommation concurrente ; erreur ultérieure ne réarmant pas la tentative.
- Préfixe capturé après overflow jamais preuve de confidentialité.
- Exit natif Chromium zéro toujours INCONCLUSIVE/nonzero/nonpublié ; table k6 distincte des catégories Chromium.
- Aucun chaînage trouvé transformant refus, cas incomplet ou diagnostic direct en qualification, publication brute ou autorité runtime.

Fixtures négatives consultées : test_direct_followup.py, test_boundaries.py, test_linux_comparison.py et test_direct_chromium.py. Simulations, pas preuve Chromium/Linux réelle.

## Contrôles et limites

Exécutés : git diff --check normal et plage source exit0 ; sept fichiers identiques à0601f6f ; préfixe job24 identique ; aucun changement libs/pins/scénarios/provisionneur.

Preuve Windows lead118tests/exit0/skips7 (111exercés) réutilisée, non rejouée. Versions k6/Chromium, deux inspect et commandes Linux validate/run/browser/selftest non exécutés dans cette revue. Aucune ressource runtime créée/démarrée, aucun Cargo, téléchargement, CI, sous-agent ou correction. AppArmor Proposed reste non autorisé ; aucun élargissement HTTP/OAuth.

Ne pas réauditer globalement les gardes inchangées ou les campagnes historiques ; ne pas assimiler ce signal privé à un bail ou à une preuve de propriété runtime. La preuve Linux/direct réelle reste à acquérir par le parent ; cette absence n'est pas présentée comme vulnérabilité.

## Fraîcheur et provenance

Revue liée au HEAD7015c8c et aux sept fichiers identiques à0601f6f ; arbre sale hors périmètre, aucune modification par le reviewer. Revérifier les locations si les fichiers changent.

Rapport READONLY `result_mvu8dwid_0012_p4gbd6`, session `ses_e75d0e1a5ffeFoHa4qhmpHfZvX`, persisté par le lead.
