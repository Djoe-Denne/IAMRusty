# Revue indépendante — fermeture du refus privé ambigu

```yaml
reviewer: security
written_at: 2026-10-30T12:54Z
branch: master
head_sha: 19b1cfe18c1cedc032b44701dd5f0032bd1ba621
dirty: true
scope_slug: k6-private-site-s1
verdict: PASS
related_paths:
  - ops/tests/k6/run-linux.py
  - ops/tests/k6/tests/test_private_site.py
```

Rapport READONLY persisté par le lead. Candidat `result_mvuz5ugb_0000_jxp7mm`,
session `ses_e732e1ec9ffefmvZm96gr2oByz`, tâche `task_mvuykugk_002f_m27x96`.
Cette fermeture statique ne vaut ni preuve Linux ni admission de collecte.

## Verdict et finding

**S-1 fermé** après examen du diff réel968592f2→cbab06eb et du code intégré.
Aucun nouveau défaut de sécurité démontré dans ce correctif limité, aucun
finding restant dans le périmètre de fermeture.

Localisation : run-linux.py:420–429.

- Le contrôle complet des deux flux par direct_text_observation précède le veto.
- La ligne425 exige une seule occurrence littérale de FATAL sur les deux buffers,
  comptés séparément.
- Le contre-exemple initial contient au moins deux occurrences : refus avant
  extraction et record, même si le second n'est pas conforme au format complet.
- Format positif et classificateur public inchangés.

## Passes de contrôle

### Défensive

Les cinq lignes ajoutées ne font qu'ajouter un refus privé. Elles ne contournent
aucune garde de confidentialité, identité, overflow ou cleanup. Aucun nouveau
champ, fichier, appel enfant ou canal de sortie n'est ajouté.

### Adversariale

Le veto ferme le contre-exemple dans le même flux ou entre flux opposés, sans
dépendre de l'ordre. Fragments sans LF, headers incomplets et marqueurs non
conformes conservant FATAL sont refusés. Une mention dans le corps ou du texte
ordinaire peut empêcher le relevé : faux négatif conservateur annoncé, pas une
nouvelle autorisation de divulgation.

### Oracles examinés

test_private_site.py:404–505 :

- runner.main et vrai Child.execute sur frontières système simulées ;
- fragments concurrents, mêmes flux, flux opposés et ordres inversés ;
- assertion que record n'est pas appelé ;
- absence du fichier privé et de result.json/provenance.json ;
- stdout/stderr contrôlés, contenu retenu et exit non nul ;
- positif unique dans chaque flux, avec exits natifs zéro et non zéro.

Aucune régression concrète identifiée dans ce delta.

## Limites et conditions

Ne pas rouvrir audit global, primitives du module ou composants inchangés. Aucun
site réel nécessaire pour confirmer la fermeture. Aucun élargissement positif,
projection publique, pin, scénario, sandbox ou permission CI modifié ; aucun
runtime ou nouvelle infrastructure.

Aucune question ouverte pour S-1. Restent distincts : résultat Linux du correctif,
contrôles du vrai parent, bail et rendez-vous de lecture. Leur acquisition
conditionne toute collecte ; cette revue ne les fournit pas.

## Vérifications réellement exécutées

- git diff --check normal et du diff correctif : exit0.
- Checkout comparé à cbab06eb sur outillage/CI : exit0, aucun delta.
- Deux fichiers modifiés confirmés.

Windows165/1.130s/exit0/skips8 : preuve attribuée au lead, non réexécutée.
Non exécutés ici : unittest, WSL, Chromium/k6, inspections, validate/run/browser/
selftest, Cargo ou CI. Aucun résultat Linux du correctif vérifié par ce reviewer.
Aucune écriture, collecte ou ressource créée/redémarrée.

Fermeture liée au HEAD19b1cfe et au correctif cbab06eb ; persister avant admission
ultérieure. Le lead rapproche séparément les preuves de tests sans les attribuer
au reviewer.
