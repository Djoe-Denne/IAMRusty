# Identifier une session privée pour poursuivre le diagnostic

## Objectif et point atteint

Nous cherchons pourquoi Chromium ne démarre pas, sans affaiblir sa sandbox ni
publier de données natives sensibles. Le dernier essai a confirmé un message au
format fatal et SIGABRT, mais pas le fichier source concerné. Les 140 tests Linux
passent sur chaque plateforme ; le navigateur et HTTP ne sont pas qualifiés.

Les reconnaisseurs ont atteint leur limite utile. Un nouvel essai identique
n'apporterait pas l'emplacement manquant. Le dépôt est public : ses journaux GitHub
ne peuvent pas recevoir cet emplacement encore non vérifié comme public.

## Recommandation

Utiliser le Linux privé que vous aviez proposé, **s'il est disponible**, et désigner
la personne qui pourra lire localement le seul fichier relatif et son numéro de
ligne. Aucun corps de message, assertion, identifiant de processus ou journal brut
ne serait conservé ou envoyé au chat.

L'architecte propose un petit relevé privé, hors Git, limité à1KiB, accessible au
seul opérateur et retiré après lecture, au plus tard sous60minutes sous surveillance.
Il ne serait créé qu'après vérification de l'hôte, des dépendances, des binaires et
des protections. Aucun paquet n'est installé implicitement. La proposition reste
Proposed : pas de code ou collecte admis avant identification de cette session et
de ce lecteur, puis ratification et vérification du mécanisme.

## Choix réalistes

- **Votre Linux privé, lecture locale par vous ou un opérateur nommé.** Meilleure
  séparation avec les journaux publics ; il faut préciser distribution/version et
  ouvrir une session accessible pour le travail. Un hôte différent peut produire
  une autre issue et ne prouvera pas rétroactivement la cause du run GitHub.
- **WSL local, lecture privée par vous.** Évite un changement de machine, mais le
  navigateur y avait échoué sur une bibliothèque absente. Le préflight et, si besoin,
  un plan explicite de dépendances restent nécessaires ; ni disponibilité ni
  qualification ne sont promises. Aucune installation n'est autorisée par ce choix
  seul, et la comparaison ne serait pas équivalente au runner GitHub.
- **Différer la recherche de cause.** Aucun nouveau relevé ou essai ; les livrables
  publiés et les preuves restent conservés. La cause et la qualification navigateur
  demeurent ouvertes ; ce n'est pas une clôture réussie de la mission.

Aucun délai fiable n'est établi pour ces options. Les garanties, pins et verdicts
restent inchangés dans tous les cas. Publier le diagnostic brut, créer un service
ou tunnel privé ou ajouter un secret CI n'est pas proposé.

## Décision précise demandée

Quelle session privée pouvez-vous rendre disponible — distribution et version —
et qui pourra y lire localement ce seul emplacement sans recopier son contenu
inconnu ici ? Ce n'est pas un nouveau GO générique de développement : il manque
un environnement et un destinataire réels que le lead ne peut pas inventer.

## Preuves et état technique

- Proposition architecte `result_mvuayb9x_0008_diwos1`, même session ; nouveau contrat
  récupéré à l'identique, `git diff --check` exit0, ancien candidat préservé.
- `docs/contracts/k6-chromium-causal-observation.md` reste Proposed/Unimplemented.
  Sa sauvegarde récupérable exacte est
  `.tmp/k6-private-observation-proposed-3818e38152db427b9928719f35f8fc19.md`.
- Résultats réutilisés du [run40985953377](https://github.com/Djoe-Denne/IAMRusty/actions/runs/40985953377) :
  140Linux/OK sans skip22/24, outils/inspect PASS, direct22 FATAL/SIGABRT avant
  cleanup ; enfants cleanup PASS et zéro artefact publié. Pas de nouveau test.
- Source `1c0036f` publiée dans `6c27e17` ; dernier distant documentaire vérifié
  `7aaaeaf` sur `origin/master`. Aucun runtime local créé/redémarré ; arbres et
  sauvegardes préservés, modifications étrangères exclues, mission non clôturée.
