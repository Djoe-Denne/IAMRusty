Je prépare une évolution de nos contrôles de disponibilité. Je veux éviter qu'un service paraisse en bonne santé alors que sa messagerie est configurée mais ne peut pas réellement fonctionner.

Explique-moi ce que fait aujourd'hui le code : comment distingue-t-il une messagerie volontairement désactivée d'un transport réellement construit ou d'un adaptateur inerte inattendu ? Comment cette distinction remonte-t-elle jusqu'à la réponse HTTP, et qu'est-ce que cela garantit réellement sur la disponibilité du broker ?

Si nous changeons demain cette classification commune, quels services et quels points d'entrée seraient concernés ? Quels chemins publics faudrait-il préserver pour ne pas casser leurs consommateurs ? Compare aussi les implémentations proches entre services : ce qui est vraiment commun, et ce qu'il serait trompeur de factoriser sur la seule ressemblance du code.

Enfin, retrouve les tests qui protègent ces comportements et les décisions documentées qui les justifient. Distingue ce qui est effectivement couvert, les cas négatifs manquants ou incertains, et les éventuels écarts entre documentation et implémentation.

Je veux une synthèse concrète, avec les fichiers et symboles qui étayent tes conclusions, puis une recommandation limitée à ce qu'on sait réellement. Ne modifie pas le code ni la configuration et ne lance pas de tests ou de déploiement : pour l'instant, je veux comprendre l'existant et le risque d'un changement.
