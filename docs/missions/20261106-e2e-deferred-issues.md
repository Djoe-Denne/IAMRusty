# Problèmes à traiter après le lancement des E2E

Décision utilisateur du 6 novembre 2026 : priorité au lancement réel des E2E
existantes. Réparer seulement les préconditions nécessaires et consigner ici les
causes de fond, sans transformer le lancement en campagne de refactorisation.
Préserver données, clés, certificats et ressources étrangères ; aucune réduction
des 39 obligations mesh ou des six Apparatus, aucun faux PASS ou nouvel essai
de rétention après consommation du seul créneau autorisé.

## À reprendre ensuite

1. **Adresse API dans les règles réseau.** Le control-plane local a changé
   d'adresse ; `allow-api` d'OpenBao conservait l'ancienne. Ajout local du seul
   `/32` actuel effectué, anciens peers conservés. Examiner ensuite le mécanisme
   de mise à jour à la reprise du lab, sans étendre l'egress globalement.
2. **Sonde CLI OpenBao.** `bao token lookup -field` n'est pas disponible dans
   l'image 2.6.2. La ConfigMap dédiée utilise les appels `bao read -field` valides,
   avec mêmes bornes de durée et refus root. Source du helper non corrigée ;
   préparer son correctif et sa régression après le lancement.
3. **Référence d'instance du coffre.** L'epoch courant diffère du registre
   d'environnement ; les deux documents GitHub n'étaient pas trouvés. Restauration
   des valeurs déjà préparées autorisée explicitement, sans rotation ou écrasement.
   Examiner ensuite la reprise du coffre volatile et le référencement de l'entrée.
4. **Renouvellement du JWT Calico CNI.** Les trois JWT de configuration CNI ont
   expiré le 6 novembre vers 16:38 UTC malgré les Pods Calico Ready. Le premier
   Job de restauration n'a pas pu démarrer et a atteint son délai. Cause de fond
   du renouvellement à analyser ensuite ; rétablissement minimal par redémarrage
   progressif du seul DaemonSet Calico existant, pas du cluster ou des applications.
5. **Diagnostic et parcours opérateur.** Le kubeconfig privé a été omis dans les
   premières commandes manuelles. Le diagnostic spécialisé ne couvre pas l'agent
   init ; aucune extension de code n'est nécessaire au diagnostic direct choisi.
6. **Vérification après création dans l'orchestration temporaire.** Une instance
   d'ancre gardait en mémoire le bail précédant la création du Job. Son contrôle
   d'ownership final a donc refusé un Job réellement terminé avec succès. Relecture
   du bail actuel et de l'UID/condition Complete effectuée, aucun seed rejoué.
7. **Valeur par défaut Kubernetes dans le préflight.** Le champ `readOnly=false`
   du writer de snapshot est omis par l'API. Le pilote comparait sa représentation
   strictement au rendu source et refusait ce montage équivalent. Correction
   compatible de la seule valeur par défaut, avec deux tests indépendants :
   omission acceptée, writer `readOnly=true` toujours refusé. ROUGE avant correction
   puis 80 tests réussis. Aucun contournement du préflight ou modification des cas.
8. **Profil d'initialisation IAM du tunnel E2E.** Le contrôle de tunnel historique
   ne connaissait que l'init OAuth de confiance et refusait le snapshot OpenBao
   déjà déployé. Adaptation minimale aux deux listes ordonnées standard IAM,
   vérification de chaque init terminé avec succès ; autres services et init
   arbitraires/éphémères toujours refusés. Tests indépendants rouges puis dix
   tests de tunnel et 76 tests d'ancre verts. Ne pas généraliser ce correctif en
   nouveau transport ; les anciennes preuves liées aux octets du helper ne sont
   pas réécrites ou assimilées à cette version.
9. **Surcoût du pilote en rétention.** Inspection source pendant le run, sans
   profilage ni modification : chaque contrôle de capacité refait le binding
   HEAD/fichiers/certificats/cible/epoch et reconstruit le reçu complet. Sous Windows,
   chaque contrôle d'ACL de fichier lance PowerShell. Le coût de cette relecture
   augmente avec le nombre d'événements. En outre, sur le cluster à trois nœuds,
   un appel utile `target.kubectl` exécute deux `local_check` : huit commandes
   Docker, deux PowerShell et le GET kube-system avant la commande utile, soit
   douze processus externes dans ce chemin normal. Ces contrôles sont répétés
   avant requêtes HTTP, opérations SQL et écritures du journal. Cela explique
   structurellement un surcoût important et constitue l'hypothèse principale de
   lenteur ; aucune attribution chronométrée exclusive au pilote n'est encore
   démontrée. Mesurer après le terminal, puis réduire les redondances sans enlever
   les garanties d'identité, anti-rejeu, absence de données étrangères et durabilité.
10. **Alignement du moteur de tests avec l'objectif utilisateur.** Le run courant
    appelle `local-full-acceptance.py` → `local_full_e2e.py` → cas Python ; il ne
    lance pas k6. Vérification approfondie : le circuit k6 livré comprend réellement
    inscription, connexion, réassociation, refus et témoins de certificat, pas
    seulement un smoke. Le lead a repris le mauvais point d'entrée. Ces scénarios
    sont encore câblés sur GitLab et le fournisseur synthétique ; leur raccord au
    profil GitHub seul reste à établir. Leur existence ne prouve pas l'exécution
    k6 des 39 cas actuels ou des six Apparatus. L'utilisateur
    rappelle la séparation attendue : environnement démarré indépendamment,
    exécuteur k6 consommant ses interfaces publiques, préparation/provenance et
    collecte aux frontières de campagne. Les vérifications Kubernetes répétées
    dans le chemin de chaque assertion ne sont pas une interaction métier avec
    le système testé. Clarifier ce raccord après le run, sans substitution à chaud,
    perte d'oracles métier ou présentation de la campagne Python comme du k6.
    Détail et décision courante : [nettoyage k6 seul](20261106-k6-only-cleanup.md).
    Les rapports de la mission sont conservés dans sa sauvegarde externe, pas
    laissés comme fichiers non suivis dans le checkout principal.

## Progression observée

- Redémarrage progressif du seul DaemonSet Calico terminé ; cluster et applications
  non redémarrés par cette opération. La cause durable du renouvellement reste différée.
- Après rétablissement du réseau, Job de restauration terminé avec succès,
  même UID revalidé sur le bail actuel, sans nouvel essai après son succès.
- GitHub puis IAM ont terminé leur activation et sont prêts. La vérification du
  profil effectif et le préflight E2E restent distincts ; aucun PASS global déduit.

Les observations et reçus précis restent dans le handoff de lancement et sous
`.tmp/github-only-restart-20261105/`. Ce registre ne constitue pas une preuve
de lancement, de réussite globale des tests ou de résolution durable de ces sujets.
