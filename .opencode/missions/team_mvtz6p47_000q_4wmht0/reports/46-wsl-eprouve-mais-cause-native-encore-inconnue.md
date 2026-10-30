# WSL éprouvé, mais cause native encore inconnue

## Ce qui est établi

L'instance dédiée permet désormais d'exécuter les outils : **140 tests locaux
réussis sans skip**, archives et exécutables conformes, versions réelles k6 et
Chromium attendues, deux inspections réussies. Les dépendances manquantes de
l'ancienne instance ne bloquent donc plus cette nouvelle campagne.

Le cas navigateur s'arrête pourtant avant son début. Un diagnostic direct unique
sur le même hôte rapporte un format fatal Chromium puis SIGABRT avant le nettoyage.
Il ne révèle ni le site fautif ni la cause. Le nouveau WSL n'est pas déclaré
équivalent au runner GitHub et ce résultat ne prouve pas une cause commune.

## Conséquence et prochaine étape

Chromium et HTTP restent **non qualifiés**. Aucun nouvel essai identique ou
reconnaisseur n'est retenu. L'emplacement source précis demeure l'information
manquante ; le canal privé proposé n'a pas été implémenté et aucune donnée native
inconnue n'a été exposée.

L'utilisateur a déjà désigné son instance locale et lui-même comme lecteur.
L'architecte reprend donc le contrat pour préciser son admission réalisable sur
l'hôte maintenant disponible, avec tests et revue ciblés avant toute collecte.
La préparation de ce mécanisme n'exige pas un nouveau GO général et n'autorise
pas une collecte anticipée. Sa rétention courte exigera une lecture locale
effectivement organisée ; aucun contenu inconnu ne sera recopié dans le chat.

## Ressources et preuves

Les huit journaux d'identités représentent 30 processus possédés distincts ;
aucun n'est encore actif après vérification de PID, UID et début de vie.
AIForAll a ensuite été arrêté de façon ciblée, exit0, état Stopped vérifié.
Son disque et ses outils sont conservés ; ARTIST-Ubuntu reste Stopped et Docker
Desktop Running. Aucun arrêt global, suppression ou ressource Docker créée.

Source exécutée : HEAD propre `05bd6c6`, outillage `1c0036f`. Résultats fermés
locaux dans le répertoire de campagne ext4 : native-unit-result.json,
native-acquisition-result.json, native-tools-result.json, native-browser-result.json,
native-direct-result.json et native-cleanup-result.json.

- Navigateur : exit2, SANDBOX_UNPROVEN, k6 SCRIPT_ABORTED, résumé valide,
  attendu1/démarré0/achevé0/itération1/requête0, aucun renderer observé.
  Confidentialité NOT_QUALIFIED, enfants cleanup PASS ; zéro fichier publiable.
- Direct : exit2 diagnostique, SIGABRT observé avant cleanup, stdout vide,
  stderr présent/CHROMIUM_FATAL_REPORTED, confidentialité et enfants cleanup PASS.
  Zéro fichier publiable ; ce n'est pas un succès navigateur.
- Métadonnées admises : AppArmor DISABLED sur cet hôte, limite userns positive,
  parent seccomp ZERO ; ces valeurs ne désignent aucune cause d'arrêt.
- HTTP et selftests réels non exécutés, toujours sans exposition autorisée conforme.
  Mission non clôturée ; aucune qualification globale ou résolution de panne annoncée.
