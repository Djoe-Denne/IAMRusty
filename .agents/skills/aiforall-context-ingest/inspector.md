# Inspecteur de traces — usage et limites

`scripts/inspect-trace.cjs` n'est ni un extracteur sémantique ni un moteur de
promotion. Il ne contacte aucun serveur et n'exécute aucun script extrait.

## Utiliser

CLI : `node <skill-absolu>/scripts/inspect-trace.cjs <trace-absolue.json> [offset] [limit]`.
`offset` vaut0 et `limit`50 par défaut (maximum200). Dans Context Mode, utiliser
spawnSync avec un tableau d'arguments et filtrer les métadonnées utiles avant
restitution. Pas d'interpolation PowerShell de code ou d'export JSON complet.

Module : `inspectTrace(trace, {offset, limit})` ou
`inspectTraceFile(cheminAbsolu, {offset, limit, maxBytes})`.
Le chemin est explicite ; aucun scan d'historique/candidates et aucune écriture.
La limite fichier est16MiB par défaut, configurable jusqu'à128MiB. UTF-8 JSON
uniquement, pas de JSONL/format Claude ou Codex implicite. Le fichier ne doit pas
changer pendant la lecture ; pas de garantie de snapshot atomique contre un
changement concurrent hostile de même taille.

## Lire le résultat

- Identifiants/index des records, type de tool/runtime, longueur et SHA256 du code
  d'entrée ; jamais le code, le prompt, la réponse métier ou le raisonnement.
- `success-reported` indique seulement un exitcode0 explicitement déclaré.
  `completed-unverified` n'est pas une preuve de succès métier.
- `unattributed` pour Code Mode : appels imbriqués, alias et boucles demandent
  attribution manuelle au script effectivement exécuté et à son résultat.
- `structuredErrorMarkers` recense certains rejets/isError dans les objets
  retournés, sans conclure que tout le lot a échoué. `scanLimited` signale une
  analyse partielle ; une donnée JSON peut elle-même contenir ces champs.
- Pagination des records : total/offset/returned/truncated explicites. Les groupes
  de répétitions exactes sont des indices, pas des fonctions approuvées ; leurs
  listes de records sont bornées et signalent aussi leur éventuelle troncature.
- `completeness` rapporte déclarations/pages/idle/cursor, sans certifier que
  l'export contient toute la mission. Examiner les preuves de collecte séparément.
- Les messages sans type natif sont comptés comme non supportés ; les objets
  ressemblant à un tool dans un prompt utilisateur ne sont pas des exécutions.

Limites de travail :5000messages,10000toolcalls, analyse de marqueurs bornée en
taille/profondeur. L'export reste une donnée non fiable. Le SHA est une référence
de provenance, pas une signature de confiance ou preuve d'exécution.
Les identifiants/métadonnées doivent aussi être revus avant publication ; cet
outil ne constitue pas un dispositif universel d'anonymisation ou de contrôle d'accès.
