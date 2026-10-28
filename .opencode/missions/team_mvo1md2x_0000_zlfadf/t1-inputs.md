# Entrées T1 — critères et isolation

Ancien worktree pool-1 (historique, ne pas utiliser pour un nouveau membre) :
`C:/Users/djden/.local/share/opencode/worktree/186749/ensemble-04d9a2f8-zlfadf-pool-1`.
Les écritures T1 doivent viser le worktree alloué au propriétaire actuellement
admis, jamais le checkout canonique par défaut. Vérifier sa racine effective et
son HEAD avant écriture ; refus d'accès
ou divergence = signalement, pas de repli vers la racine canonique.

## Critères convenus dans la conversation

### AC1 — Atomicité des deux chemins de création

Pour une création réussie : exactement une notification, exactement une
livraison lorsque demandée, exactement une ligne outbox correspondante, payload
contenant les bons notification_id et user_id. Une panne d'insertion de livraison
ou d'enregistrement outbox laisse zéro nouvelle ligne métier et zéro événement
pour cette tentative. Preuve sur PostgreSQL réel, pas uniquement sur mocks.

### AC2 — Panne du transport après validation

Avec le publisher réellement indisponible, la création réussit dès validation
de la transaction locale ; l'événement reste durablement non publié et le
dispatcher conserve une possibilité de reprise. L'échec de publication ne
provoque pas à lui seul une nouvelle création métier.

### AC3 — Reprise sans mémoire du premier dispatcher

Après commit, arrêter le dispatcher avant publication et créer une nouvelle
instance sur la même base : publication du même événement avec le même event_id,
transition vers published après succès, aucune notification ou livraison
supplémentaire créée par cette reprise.

### AC4 — Pas de faux succès no-op

Contrat proposé à ratifier : transport désactivé ou dégradation vers no-op =
suspendre la publication et conserver les événements non publiés. Un Ok(())
provenant d'un publisher no-op ne suffit jamais à les marquer published.

### AC5 — Preuve fonctionnelle jusqu'au droit réel

Sans openfga.allow préalable : produire user_email_verified ; constater la
notification et l'intention outbox ; recevoir NotificationCreated sur la vraie
file SQS de test ; le traiter par le vrai SyncEventHandler vers OpenFGA réel ;
obtenir HTTP 200 au marquage comme lu par le destinataire, HTTP 403 pour un
autre utilisateur sans modification par celui-ci. Même résultat après une panne
temporaire de publication.

### AC6 — Livraison répétée sûre côté aval

Présenter deux fois le même NotificationCreated : droit du destinataire correct,
aucun droit attribué à un autre utilisateur, aucune erreur empêchant la
convergence. Garantie at-least-once, pas unicité globale des traitements.

### AC7 — Cycle de vie maîtrisé

En standalone et monolithe : consommateur et dispatcher supervisés, erreurs non
silencieusement perdues, arrêt dans une échéance fixée par le contrat QA, aucune
tâche laissée détachée après shutdown.

## Profil et permissions

L'ancien profil admis build a été confirmé par pool-1. Le harness interdit le
changement de profil d'une tâche autonome ; un message du lead ne l'autorise
pas. L'utilisateur a autorisé une réparation d'affectation : pool-1 est arrêté
et T1 remise pending par le shutdown natif. Le lead tente une nouvelle admission
explicite sous architecte, sans override modèle. La nouvelle session doit
confirmer son profil effectif avant rédaction. Aucun modèle, profil ou stockage
interne à contourner.
Pas besoin de fork, de script Context Mode ou de référence externe de continuité
pour rédiger le contrat documentaire T1. Si une telle opération devient utile,
signaler le besoin et les permissions manquantes avant exécution.
