# Reprise : préparer la dernière mesure de dépendances, sans Chrome

L'objectif reste de réussir le cas navigateur avec son isolation active. Chrome
n'est toujours pas qualifié ; les27tests verts portent sur le diagnostic, pas
sur le navigateur. La création d'un espace de noms Linux a réussi, mais le
premier contrôle de dépendances est resté inconclusif.

Le défaut du lecteur est corrigé et vérifié. Le lead a maintenant vérifié les
deux documents reçus de l'architecte et ratifié uniquement l'exception permettant
une seconde et dernière mesure ldd. La proposition de correction du conducteur
Windows reste Proposed, et la collecte privée reste interdite.

## Ce qui est lancé maintenant

Le même auteur prépare la sélection temporaire « ldd seulement » et son quota
persistant. Aucun second test d'isolation, lancement Chrome/k6 ou changement de
sécurité. Le quota restera consommé après une erreur ou un changement de bail ;
aucune boucle de répétition n'est admise. Les sorties privées restent bornées,
et les anciennes preuves ne sont pas réécrites.

Le parent vérifie en parallèle les entrées et l'inventaire nécessaires à la
suite et à la publication. Aucun runtime n'est réveillé pendant cette préparation.
Les modifications d'autres missions, le sous-module et les données utilisateur
sont conservés et exclus du lot. Pas de nouveau GO de jalon demandé : cette
étape relève du mandat déjà approuvé.

Après intégration et tests ciblés du raccord, une seule mesure pourra être
consommée sous bail neuf. Si elle reste inexploitable, pas de nouvel ajustement
du lecteur suivi d'un nouveau run : le repli Linux natif sera instruit avec ses
conditions réelles. Aucun nouveau système ou hôte n'est choisi par ce rapport.

## Preuves et limites

- HEAD principal `9b2d8f14b86a09d11f983f9a34bd2271f384874b`, OPS inchangé depuisf279.
- `git diff --check` global exit0, index vide ; changements étrangers préservés.
- Livraison architecte `result_mvv8umt6_000q_earwag` identique octet pour octet aux
  deux fichiers canoniques avant ratification ; LDD SHA256cf9b4231…, Windowsec0c769e….
- Contrat ratifié : `docs/contracts/k6-ldd-parser-completion.md`, Accepted/Unimplemented.
- Rapport73 :27tests/exit0/sansskip déjà acquis et réutilisés. Aucune nouvelle
  suite, mesure Linux, vérification HTTP ou exécution navigateur à ce stade.
- Bail historique libéré et AIForAll observée arrêtée lors du dernier inventaire ;
  ce fait sera recontrôlé avant toute nouvelle opération.
