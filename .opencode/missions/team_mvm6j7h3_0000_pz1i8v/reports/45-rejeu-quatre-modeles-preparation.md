# 45 — Nouveau scénario, quatre modèles : préparation

## Mandat et stade
L'utilisateur demande de reprendre les trois modèles du dernier lot et d'ajouter
Solmedium sur le nouveau scénario proposé. Exécution directe par sessions natives,
pas d'équipe ou d'agent intermédiaire. Préparation en cours, aucune tâche lancée.

## Modèles
- openai/gpt-6-luna#high.
- openai/gpt-6-luna#xhigh.
- opencode-go/claude-haiku-5-5#medium.
- openai/gpt-6.1-sol#medium.
Références/variantes déjà vérifiées au catalogue ; contrôle de nouveau dans les
sessions créées et leurs messages effectifs. Aucun fallback automatique.

## Méthode
Même nouveau prompt métier naturel pour les4sessions, sans nom d'outil ni chemin
source à analyser. Pas de préexploration du parcours ou d'oracle technique créé
avant les essais. Prompt et contrôleurs conservés hors checkout ; ce document
ne fournit pas de réponse ni de grille technique aux sessions.

Lecture seule : aucune modification, compilation, test ou infrastructure.
Agentbuild, aucun parent, zéro message avant envoi, permissions ordinaires sans
--auto. Vérifications de prompt, modèle, sources suivies et absence des12anciens
fichiers d'audit avant envoi. Pagination correcte, état/PID privés persistants.
Lanceur refuse une session non vide ou un enregistrement existant.

Quatre lancements background avec notifications ; aucun polling ni guidage
pendant l'exécution. Consignes gelées ; pas de conclusions accessibles au dépôt
avant fin des4. Le bilan détaillé restera hors checkout pour les prochains essais.

## Limites et validation
Observer les appels réels, la qualité des preuves et le respect du mandat sans
exiger artificiellement un MCP. Nouveau sujet : les temps ne sont pas comparables
causalement aux anciens essais. MCP/index/caches et charge partagés entre modèles,
une exécution par cas ; pas de benchmark isolé ou économie financière garantie.
Pas de nouvelle configuration, runtime, Cargo, Docker/Kind/WSL ou déchargement8B.

## Préflight final avant lancement
- Nouveau prompt :914caractères après retrait de la seule fin de ligne de fichier,
  SHA256 f9e66c1ac62d61a409c86980d93e09a1f3c9c47e91fa2100681ca5d7347213f9.
  Même texte pour les4, pas de nom d'outil, fichier, symbole ou solution fournis.
-12anciens chemins d'audit toujours absents, nouveau dossier de campagne distinct.
- Lanceur privé vérifié node--checkexit0 ; campagne replay séparée des anciens
  états, pagination correcte, refuse de renvoyer un prompt après enregistrement.
- Sessions créées/vérifiées build, aucunparent et0message initial :
  - Luna high : ses_e8d01ba97ffetOsHMf0S4GtN8v.
  - Luna xhigh : ses_e8d01b9bdffehsKW3H5KXZO5Hs.
  - Haiku Go medium : ses_e8d01b8e7ffeLBLmM3wjJZaTVE.
  - Sol medium : ses_e8d01b809ffeagDHHbOTZgFylP.
- Les lancements suivent cet enregistrement ; prochains résultats détaillés
  seulement horscheckout après les4fins. Aucun scénario précédent rejoué.
