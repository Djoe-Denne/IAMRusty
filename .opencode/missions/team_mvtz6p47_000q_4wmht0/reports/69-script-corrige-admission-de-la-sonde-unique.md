# Script corrigé : admission de la sonde unique sans navigateur

L'enquête reprend les deux pistes de l'utilisateur : espaces de noms et dépendances
Linux. Le classement des signaux est corrigé, sans changement des outils, du
sandbox, de la CI ou des sources k6 existantes. La régression du lead est verte.

## Vérifications acquises avant runtime

- Delta natif reçu et appliqué par patch, nouveaux SHA256 identiques à l'auteur.
- Suite ponctuelle lead :24tests/0.239s/exit0/sansskip. Elle comprend19tests auteur,
  la régression indépendante du signal et quatre fixtures des copies/retraits.
- Les fixtures de fichiers prouvent notamment la conservation du clone Git
  initial, le refus d'une source modifiée, la préservation d'un remplacement
  étranger et le maintien des preuves si le run ne peut pas être retiré.
- Le parent Windows réutilise les API natives déjà revues de handle/processus et
  de lecture bornée ; aucun conducteur privé ou préflight de celui-ci n'est appelé.
  Vérification native sur le propre processus Windows et mémoire synthétique :
  exit0, aucun runtime démarré. L'enveloppe publique maximale construite selon
  les bornes du schéma fait2364octets, sous la limite4096 du lecteur réutilisé.

## Opération admise, pas encore résultat

Un bail neuf sera créé avant le réveil d'AIForAll, avec inventaire et identité
GUID/nom/emplacement/UID/SID observés. Les quatre modules publics et le script
seront copiés et hashés dans un répertoire privé neuf hors Git. Le clone initial
et les outils ne seront ni modifiés ni déplacés.

Une invocation du script lance seulement le contexte enfant, une commande userns
fermée et une commande ldd du binaire/chargeur vérifiés, via Child inchangé.
Chaque sonde est bornée à10s, sans retry. Le parent laisse le nettoyage se terminer,
joint ses propres handles Windows, puis retire les copies par leurs identités.
Il n'utilise pas la fin d'un processus WSL comme preuve du nettoyage Linux.

Aucune sortie native brute publiée. Publication interdite si confidentialité,
identités ou nettoyage non prouvés ; toute mesure manquante reste inconnue.
Le script ne teste pas les commandes dans un vrai terminal utilisateur et ne
qualifie ni HTTP, ni Chromium. Aucun Chrome/k6, réseau applicatif, paquet, privilège,
sysctl, AppArmor, TLS ou permission de stockage modifié.

En fin d'opération, AIForAll sera arrêtée seulement si les invocations possédées
sont jointes, les copies retirées, le nettoyage Linux prouvé et aucun autre
processus utilisateur observé. Sinon le bail/état restant sera signalé sans arrêt
étranger ou forcé. Docker et les autres instances restent hors de cette opération.

## Annexe technique

- Contrat Accepted : `docs/contracts/k6-chromium-linux-probes.md`.
- Source durable inchangée : `f279e713079159766188fcbe5078d37217ca530d`.
- Candidat corrigé `result_mvv6twwr_0002_co10dc` ; script SHA256
  `7f17cdc551a62cc3b73c513da961677ed7316ff34332d1777b3c795961f64554`,
  fixtures `09da2edea06721396d7df89e4b72d2b0accfed2b31b87d6f31456502b2a2074d`.
- Scripts temporaires du parent : `.tmp/k6-probe-parent-20261030.ps1` et
  `.tmp/k6-probe-parent-files-20261030.py`, non destinés à être publiés comme un
  nouveau framework. Tests/lecture avant usage, aucune ancienne identité adoptée.
- Commande prévue : PowerShell natif, `k6-probe-parent-20261030.ps1 -Mode run`.
  Ce rapport d'admission ne prétend pas que la commande ou les sondes ont réussi.
