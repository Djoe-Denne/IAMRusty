# Script vérifié : corriger le classement des signaux avant les sondes

Le but est de mesurer les restrictions Linux et les dépendances sans relancer
Chrome. Le code ponctuel est reçu par livraison native et ses deux empreintes
sont confirmées. Les18tests de l'auteur passent aussi chez le lead, sans skip.

La revue a relevé un défaut limité au script : un processus arrêté par signal
est étiqueté `EXITED`. Le résultat reste non réussi, mais cette étiquette serait
trompeuse dans une enquête de démarrage. Un test synthétique indépendant échoue
exactement sur `EXITED != SIGNAL`, avant correction. L'auteur existant corrige
ce point et son schéma de sortie ; aucun nouveau framework ou rôle QA ajouté.

**Ce défaut n'explique pas l'arrêt de Chrome.** Aucune commande `unshare` ou
`ldd`, aucun navigateur ou conducteur réel n'a été lancé pour cette revue.

Une autre précondition est identifiée statiquement : le script refuse une source
dans Git, alors que le snapshot fourni est un clone. Le lead prépare donc un
export privé neuf des quatre seuls modules publics hashés, hors Git. La garde
ne sera pas relâchée et le clone/outillage existants resteront inchangés.

L'outil Context Mode refusait la lecture directe du dossier externe de l'auteur.
Le code a été reçu par le canal de livraison natif, puis transféré à l'identique
dans le projet, sans modifier les permissions ou relire le chemin refusé. Aucun
diagnostic natif privé n'est passé par ce canal.

## Preuves et suite

- Lead :18tests/0.118s/exit0/sansskip ; régression1test/0.005s/exit1 attendu.
- Candidat natif `result_mvv6l21b_0002_hn28nk`, hashes de réception7d1db94e/af4690ae.
- Brief complet : `.cursor/review-briefings/20261030-k6-linux-probes-lead-review.md`.
- Après correction : vérification du delta et des nouveaux hashes, test vert de
  la régression, préparation des entrées, puis bail neuf pour les sondes. Pas
  d'assimilation du contexte WSL non interactif à un terminal utilisateur.
- Dernier runtime connu : AIForAll arrêtée ; étrangers inchangés. Chromium et
  HTTP non qualifiés, collecte privée toujours conditionnelle et non autorisée.
