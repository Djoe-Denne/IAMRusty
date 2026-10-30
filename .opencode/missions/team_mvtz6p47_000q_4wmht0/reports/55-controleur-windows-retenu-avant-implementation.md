# Contrôleur Windows retenu avant implémentation

## Objectif et choix

Le mécanisme privé doit permettre à l'utilisateur de lire localement le seul
emplacement source de l'arrêt Chromium, sans l'exposer aux outils enregistrés ou
à GitHub. Le consommateur est testé, mais le programme qui garantit les contrôles
réels et le retrait du relevé reste à construire.

Le plan a identifié une réserve précise : il ne démontre pas encore la propriété
et la fin des processus Windows lancés depuis Linux par l'interopérabilité WSL.
Il ne prouve pas que cette voie soit impossible ; aucun probe n'a été lancé.

Le lead retient l'autre composition déjà prévue au contrat : **un contrôleur
Windows garde le handle de sa propre invocation WSL**, c'est-à-dire la référence
native permettant d'en vérifier l'identité et la fin. Les seules métadonnées de
contrôle passent par leurs pipes privés, les canaux du processus créé. Aucun
auxiliaire Windows d'interopérabilité n'est nécessaire. La coordination est un
peu plus explicite, mais la propriété du processus commence à sa création.

## Suite et garanties

Le choix et le plan reçu sont maintenant persistés. L'architecte précisera le
contrat sur cette voie, sans nouvelle recherche globale ; l'auteur existant
préparera ensuite le conducteur et ses tests dans la même session, avec une
spécialité adaptée au travail Windows/Linux.

Le site et le corps du diagnostic ne traverseront ni le contrôleur ni les pipes.
Le parent conservera les contrôles d'identité, stockage privé, bail, provenance,
fraîcheur et retrait. Aucun nouveau service, déployeur ou framework n'est ajouté.

**Aucune capture ni relance Chromium n'est autorisée à cette étape.** Les tests et
un préflight des observateurs sans armement précéderont la revue du conducteur,
puis le véritable créneau de lecture locale à convenir avec l'utilisateur.

## Preuves et limites

- Plan reçu : `result_mvuzytnb_0000_f3igv6`, persisté dans le briefing de plan auteur.
- Contrat actuel : opération Accepted pour préparation/vérification ; alternative
  pipes déjà décrite, sélection à préciser avant code.
- HEAD local vérifié `5ca37b6`, diff-check exit 0 ; code du consommateur `cbab06e`.
- Les 165 tests Linux et S-1 fermé ne valident pas encore le conducteur absent.
- Sept worktrees de mission préservés ; SDK inchangé. Aucun runtime lancé ou site
  créé, pas de push, aucune modification des travaux étrangers.
