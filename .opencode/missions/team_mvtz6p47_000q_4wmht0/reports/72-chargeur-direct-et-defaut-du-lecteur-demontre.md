# Un défaut du lecteur de dépendances est démontré, pas une cause Chrome

La lecture statique du binaire vérifié confirme32dépendances directes, dont le
chargeur Linux. Aucun `ldd`, `unshare` ou navigateur supplémentaire n'a été lancé.

L'analyse du code démontre un défaut du lecteur : une ligne de chargeur reconnue
est comptée comme chargeur mais pas comme nom de dépendance. Si ce nom figure
parmi les besoins directs de l'ELF, le lecteur peut donc refuser un résultat
pourtant complet. Une fixture connue suffit à montrer cette asymétrie.

Cela rend l'explication pertinente pour notre binaire. **La sortie ldd effacée
n'est toutefois pas reconstituable : nous ne déclarons pas que cette branche
était la seule cause de l'inconclusif, ni que les bibliothèques sont présentes.**
Le résultat réel précédent reste countnull. La cause du FATAL Chrome reste inconnue.

Le correctif minimal et ses tests sont confiés au même auteur, sans changement
de garde, de pin ou de source durable k6. Il n'autorise pas un nouvel essai : la
limite d'une commande par voie reste en vigueur et devra être arbitrée avant
toute expérience supplémentaire. L'option Linux natif reste un repli, pas lancée.

## Preuves et ressources

- AnalyseRO auteur `result_mvv8a2cg_000h_ijnump`, aucune nouvelle commande exécutée
  par lui ; sources publiques glibc2.39 recoupées, sans assimilation au runtime.
- Lecture ELF seule sous bail neufbe0cbc5a121d4324b51a53c5abdf0dc5 : métadonnées
  exit0,32besoins/chargeurDirectTRUE/pins et4sources vérifiés,0commande externe,
 0fichier de sortie créé par le lecteur, schéma allowlisté validé.
- Quatre handles Windows joints/identifiés, cinq copies publiques retirées,
  AIForAll finalement observée arrêtée ; étrangers inchangés, aucun stop explicite.
  Parent exit2 initial dû à l'observation runtime encore en marche, conservé.
- Bail libéré après l'observation finale ; aucune nouvelle ressource Docker/Kind,
  données/outils/clone préservés. Métadonnées ne qualifient ni HTTP ni Chromium.
- Brief de correction : `.cursor/review-briefings/20261030-k6-ldd-loader-parser-fix.md`.
