# Examiner les pistes utilisateur : espaces de noms et dépendances

## Reprise et résultat attendu

L'utilisateur lève sa pause pour utiliser les recherches qu'il apporte : priorité
aux restrictions de namespaces et aux bibliothèques absentes ; si cela ne débloque
pas le diagnostic, envisager Linux natif. Il évoque aussi, sans choix arrêté,
k6 Windows avec Kind dans Docker. Aucun changement de cible de qualification,
pin, sandbox, permissions ou configuration système n'en est déduit.

Consultation architecte dans la même session12, sans nouveau modèle, worker ou
permission. Mission1, ADR0504 et V1 Accepted/Partial restent l'autorité.
Canon : checkout principal `C:/Users/djden/source/repos/AIForAll`, HEAD
`0f32e8dab504458074a353cb555139801db81487`, contrat causal actuel. Lire les
sections utiles de `k6-functional-linux.md` et
`k6-chromium-causal-observation.md`, notamment l'exclusion de strace/unshare à
la ligne530 et les obligations de collecte conditionnelle. Ne pas réintégrer
les149lignes du candidat de disposition du stockage ni le déclarer Accepted.

## État et questions bornées

Chrome for Testing156.0.8078.73, k6 2.3.0, Ubuntu24.04/WSL2 amd64 non-root.
Historiques : k6 avant cas, direct sans k6 FATAL/SIGABRT avant cleanup ; cause
inconnue. AppArmor local désactivé, seccomp parent0 et limite userns positive
n'établissent pas qu'une création userns effective réussit. Aucun ldd réalisé.

Le correctif S-1/S-2 des trois fichiers du conducteur est présent mais non
committé, non vérifié par le lead et non fermé indépendamment. La vraie collecte
et le préflight de ce conducteur demeurent interdits. Le stockage réel refuse
les gardes actuelles ; pas d'assouplissement, de handles compensatoires, de
changement d'ACL ou de déplacement. Cette limite n'est pas une cause de Chrome.

Décider si un **diagnostic ponctuel sans navigateur ni site privé**, sous bail
WSL neuf, peut vérifier séparément :

- distribution, version du noyau et WSL ; états userns/seccomp/NoNewPrivs réels ;
- `unshare --user --map-root-user -- /usr/bin/true`, non-root, borné, aucun
  sysctl/privilège/helper persistant ; résultat fermé/exit réel, sans stderr brut ;
- `ldd` de l'ELF Chrome déjà épinglé et revérifié, stdout privé analysé localement,
  uniquement code de retour et compte de dépendances absentes exportés ; échec
  ou sortie indécodable jamais « aucune dépendance manquante » ;
- comparaison d'une invocation directe WSL non interactive et d'un enfant par
  les **mêmes Child/child_environment existants**, sous les mêmes identités et
  limites. Ne pas présenter la première comme un vrai terminal utilisateur, ni
  un succès unshare comme preuve de tous les namespaces nécessaires à Chrome.

Pas de nouveau framework, mode CLI permanent ou modification de Child. Un script
ponctuel sous `.tmp`, revu/testé, paraît suffisant si admis ; pas de runtime par
le worker. Préciser l'autorité et les oracles utiles avant exécution par le lead.

## Sources à recouper

- Chromium versionné156 :
  https://chromium.googlesource.com/chromium/src/+/156.0.8078.73/docs/security/apparmor-userns-restrictions.md
- Même document main, référence générale non substituée à la version épinglée.
- https://github.com/anthropics/claude-code/issues/103811 : signalement retrouvé,
  userns0 dans le sandbox de Claude Code contre30997 dans un terminal ordinaire.
  Ce n'est pas notre client ni une mesure de notre hôte.
- https://pptr.dev/troubleshooting : dépendances, limites WSL et AppArmor.
- Canonical security-overview fourni : première lecture Defuddle a expiré ; ne
  pas annoncer son contenu vérifié sans lecture réussie par la suite.

Le lead vérifie les pages en parallèle ; ne pas répéter une vaste recherche.
La mise à jour156.0.8078.80 n'est pas adoptée. Aucun texte FATAL inconnu, tail de
journal, --enable-logging, --v=1, dump/core/trace ou nouvelle tentative Chrome.

## Livrable et ownership

Avis court avec Localisation / Impact / Limites et décision de compatibilité.
Si l'exclusion précédente exige une précision contractuelle, écrire seulement
un **nouveau** `docs/contracts/k6-chromium-linux-probes.md`, Proposed, qui borne
ce diagnostic et ses garanties sans modifier les contrats Accepted ou l'ancien
candidat stockage. Le lead examinera et ratifiera si conforme au mandat.
Aucune production, test, CI, commande WSL, Cargo, collecte, installation, commit,
push, mutation du stockage ou ledger partagé ; aucun sous-agent. Les résultats
restent non qualifiants. Les choix Linux natif/Windows sont des replis à documenter,
pas une migration à implémenter maintenant.
