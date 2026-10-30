# Canal local ratifié avant code et tests

## Objectif et décision

Les essais établissent un arrêt fatal de Chromium, pas son emplacement source.
Le contrat de l'architecte est maintenant intégré et ratifié pour préparer le
petit mécanisme de lecture privée déjà proposé. L'utilisateur et son hôte sont
connus ; il n'est pas nécessaire de redemander un GO général ou de choisir un
nouvel environnement.

**Aucune collecte n'a lieu maintenant.** Le code et les tests peuvent avancer
sans lecteur présent. Avant de créer un relevé, il faudra une revue indépendante
favorable, les contrôles de l'hôte et un créneau de lecture locale réellement
convenu, afin de retirer le relevé après lecture ou sous60minutes.

## Garanties conservées

L'activation sera explicite et locale, désactivée par défaut et refusée dans
l'intégration continue. Seul un emplacement source relatif et son numéro de
ligne pourront rejoindre un fichier privé, jamais le corps du diagnostic.
Le fichier restera sur le disque Linux natif, hors Git, avec création exclusive
et accès limité au compte de travail. Aucun contenu inconnu ne sera envoyé au
chat, aux journaux ou à GitHub.

Les sorties actuelles, les exits non-zéro et les verdicts Chromium/HTTP restent
inchangés. Ni un fichier prêt ni un emplacement reconnu ne deviendra une preuve
de démarrage ou de cause racine. La projection d'une ancre publique reste Proposed.
La suppression autorisée se limite au relevé temporaire possédé ; les outils,
l'image et l'instance demandée restent conservés.

## Mise en œuvre et vérification

Un seul auteur reprend l'outillage couplé. Les tests porteront sur l'extraction
exacte, l'activation, les permissions, les liens, la taille, les erreurs et la
non-divulgation sur chemins positifs et négatifs. Une revue indépendante ciblée
du canal précédera toute capture réelle ; pas de nouveau serveur ou framework.

Le raccord local ne devra pas exiger de rejouer le cas navigateur : son refus
déjà observé sera corrélé par le parent aux pins et identités du futur direct.
Un manifeste seul ne sera pas pris pour preuve d'autorisation de l'hôte. Aucun
essai répétitif, modification de sandbox ou diagnostic privé anticipé n'est admis.

## Preuves et état

- Addendum architecte de87lignes récupéré byte-identique ; diff-check des deux
  arbres exit0, sauvegarde récupérable avant ratification.
- Sauvegarde : `.tmp/k6-private-consumer-contract-proposed-c5b9f4f4d1f74c64a72bd196c9740fb7.md`.
- Canon : `docs/contracts/k6-chromium-causal-observation.md`, addendum local Accepted,
  consommateur Unimplemented, collecte conditionnelle, ancre publique Proposed.
- Preuves locales et publication précédentes conservées au rapport46 ; runtime
  AIForAll arrêté et aucun relevé privé existant. Mission non clôturée.
