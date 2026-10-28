# Rapport 10 — réparation ciblée et reprise T1

## Objectif et autorisation

Sur mandat utilisateur, réparer la mission et reprendre avec les bons profils,
sans changement du plan, des modèles approuvés ni des règles d'architecture.

## Opérations et preuves

- pool-1 (build) arrêté nativement ; branche préservée par le harness. T1
  redevenue pending, aucune branche ou tâche dupliquée.
- team_spawn a réaffecté la tâche existante task_mvo2evlb_0003_oigv2n à
  t1-architecte sous le profil architecte. Catalogue : Sol xhigh, sans override.
  Les appels invalides précédant l'admission n'ont créé aucun membre.
- Le membre a confirmé le profil correct mais a rencontré external_directory
  en lisant .git et les docs de son worktree. Aucun livrable ni commande runtime.
- team_view a confirmé la session ses_e8bf4690cffeQakkQ72dTWT9Jc.
- opencode.session_move a confirmé son déplacement vers
  C:/Users/djden/.local/share/opencode/worktree/186749/ensemble-04d9a2f8-zlfadf-t1-architecte.
- La même session a reçu un GO explicite de reprise via team_message(force:true),
  avec préflight relatif de .git/index ADR/HEAD avant toute écriture.

## Limites et suite

Le succès du déplacement ne prouve pas encore que les outils voient la nouvelle
racine : confirmation attendue du membre. L'avertissement initial du harness
sur les permissions Ensemble n'est pas supprimé ni considéré comme résolu par
hypothèse ; le membre a néanmoins pu envoyer son diagnostic par team_message.

Aucun profil, permission, plugin ou réglage global modifié. Aucune suppression,
stash/reset, compilation, runtime ou dérogation architecturale. T2 reste bloquée
jusqu'au contrat T1 réellement intégré. Pour les prochains membres, vérifier
profil explicite et racine native allouée avant travail ; aucune substitution
silencieuse par build. Attendre le retour du membre, sans polling de progression.
