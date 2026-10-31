# Lecteur Admin revu, préparation d'une mesure réelle

La revue sécurité indépendante valide le lecteur borné et son adaptateur de preuve,
sans défaut de sécurité démontré. Le défaut du port local Kubernetes est fermé
par une régression rouge puis verte. Les 40 tests simulés et les trois unités de
l'adaptateur/prérequis passent ; ce ne sont toujours pas des preuves réseau.

Avant toute lecture réelle, je livre la même source qualifiée dans le checkout
effectivement importé. Le scénario refusera une autre postimage avant de sonder
le cluster. Le code applicatif et les images déjà livrées restent inchangés.

La première mesure réelle sera courte : état Envoy et compteurs fermés, puis
fermeture du transfert temporaire. L'interface Admin complète est brièvement
accessible aux processus du poste de confiance ; ce risque local est explicitement
accepté. Aucun navigateur OAuth ne sera actif pendant cette mesure.

La contradiction entre l'entrée hôte prévue et la règle réseau limitée au client
interne reste à résoudre par l'architecte, avec preuve du trajet avant correction.
Aucun certificat ou droit réseau n'est modifié par cette mesure.

## Limites

- Pas de réussite SAN ou navigateur annoncée.
- Source qualifiée à livrer et hash à vérifier avant mesure.
- Compteur absent : INCONCLUSIVE, jamais zéro ou succès implicite.
- Fermeture réelle et budgets Windows encore à constater.

## Première mesure réelle

La lecture atteint Admin, puis refuse une réponse dépassant sa limite de taille.
Le transfert possédé est fermé et son marqueur a disparu. Aucun corps brut ni
compteur incomplet n'a été publié ; le résultat reste INCONCLUSIVE.

Je localise maintenant lequel des deux endpoints fermés dépasse sa borne, en
ne conservant que son nom fixe et le nombre d'octets. Aucune limite n'est relevée,
aucun certificat ou droit réseau modifié pour forcer un succès.

L'endpoint concerné est désormais identifié : `/server_info` dépasse la borne
initiale de16Kio. Son handler versionné renvoie le nœud complet et ne lit pas de
masque de réponse. Les mesures identiques sont arrêtées ; aucun plafond n'a été
relevé. L'architecte examine un ajustement fixe et borné, qui devra être ratifié
et revu avant code. Il n'y aura pas de relèvements successifs jusqu'au succès.

La future correction réseau reste également conditionnée à une preuve du hop
effectif. Une règle CP/32 attendue par la documentation ne suffit pas à démontrer
le rejet observé ; aucune ouverture réseau d'essai n'est autorisée par ce contrat.

## Amendement borné décidé après ces mesures

L'architecte a proposé et le lead a ratifié un plafond **fixe de128Kio** pour
server_info. Ce n'est pas une estimation de taille ni une promesse de succès.
Les délais, les sorties réduites et la limite des statistiques restent inchangés.
Davantage de données opaques peuvent transiter en mémoire, jamais être publiées.

Si cette nouvelle borne est dépassée : arrêt et INCONCLUSIVE, sans essai à une
borne supérieure. Les tests de frontière rouge puis verte donnent42PASS ; la
revue ciblée du delta précède sa livraison et l'unique nouvel essai autorisé.
Aucune nouvelle lecture réelle n'a encore été lancée sous cet amendement.
