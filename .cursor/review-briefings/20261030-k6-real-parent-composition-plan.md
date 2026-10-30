# Plan de composition du parent réel — lecture seule avant code

## Mandat et canon

Même auteur13, même mission1 et politique approuvée. Le consommateur est intégré
et gelé à cbab06e, testé Windows165/skips8 et Linux165/0skip, S-1 fermé par revue.
Lire le contrat causal ACTUEL du principal, notamment la précision opérationnelle
Accepted pour préparation/vérification, ainsi que le rapport54. Ne pas utiliser
les vieux contrats ou HEAD physique b316 comme base à remettre au goût du jour.

Livrer un **plan technique court, sans écrire de code ni lancer de runtime**.
Le programme parent compose PrivateSiteSession et runner.main existants ; il ne
devient pas un nouveau superviseur, scénario, service, mode de qualification ou
canal d'autorité. Un programme mince et des tests du framework existant sont
préférables à une nouvelle bibliothèque générale ou à des scripts non vérifiables.

## Composition à préciser

- Voie préférée du contrat : parent Linux non-root dans une invocation explicite
  du GUID AIForAll ; contrôles Windows renouvelés par interop PowerShell RO fixe.
  Disponibilité encore à vérifier, sans modifier WSL globalement ou installer pour
  la contourner. Alternative de métadonnées sur pipes décrite, pas fallback silencieux.
- Détailler qui lance le conducteur, quel terminal privé est réellement prêt et
  comment `LU <run_id>` atteint le contexte vivant. Les outils de l'agent ne sont
  pas un terminal privé de lecture ; aucune donnée du site dans chat/MCP.
- Contrôle Windows réel : GUID/nom/BasePath/DefaultUid, identité/ACL dossier et
  VHDX, compte Windows attendu, bail courant et créneau. Aucun manifeste seul
  comme preuve. Identité et fin des auxiliaires Windows tracées à leur création,
  sans les déduire du nettoyage Linux, pas de récupération ou kill par nom.
- Contrôle Linux réel : HOME ouvert/tuple dev,ino,uid et ext4, UID/modes/chemins,
  provenance, refus nominal24 déjà conservé et intégrité des outils. Pas de nouveau
  browser pour fabriquer un reçu, ni un reçu22 fictif. Le refus historique a
  NOT_QUALIFIED et ne contient pas hashes/versions ; les preuves séparées existent.
- AUTHORIZE et WRITE : séquence, IDs et deadline exacts, observation Windows
  renouvelée chaque fois et retour sous30secondes. Refus si lent, absent, changé
  ou inconnu. Pas de couche RPC/réseau, token déclaratif ou vue atomique imaginaire.
- Maintenir le contexte après main non-zéro ; état READY seulement autorise une
  lecture par l'utilisateur. Ne pas lire le site dans le parent. Retrait après LU
  exact ou échéance UTC/monotone, sans faux True, puis vérification de retrait,
  jonction des enfants et libération du seul runtime devenu inutile.

## Données, chemins et oracles

Proposer les fichiers minimaux utiles sous ops/tests/k6 si un conducteur durable
est justifié, ou expliquer le caractère strictement jetable d'une composition
ponctuelle. Pas de chemin personnel ou GUID de cette installation transformé en
configuration universelle ; liaison d'hôte explicite et réellement revalidée.
La portée reste un parent local de cet hôte déjà autorisé, pas un sélectionneur
libre de cibles ou un déployeur. Les données de campagne restent privées hors Git.

Indiquer les commandes de vérification réelles possibles avant toute capture :
tests du framework existant, négatifs sur Windows/HOME/bail modifiés, réponses
absentes/lentes, IDs/deadline étrangers, acquittement absent/incorrect, retrait
impossible et auxiliaire Windows non joint. Aucun serveur de mocks ou harness
nouveau. Le préflight interop pourra être vérifié en lecture seule après accord
du plan, sous bail, jamais avec Chromium ou site réel.

Décrire les limites opérationnelles restantes honnêtement, surtout l'interaction
locale, la supervision jusqu'au retrait et les permissions de fin. Ne pas présenter
une fonction retournant True comme une implémentation du parent.

## Hors périmètre de ce tour

Pas d'édition source/docs/contrat/ledger, WSL, Chrome/k6, CI, installation, Cargo,
commit, push, collecte ou subagent. Le lead persistera le plan puis autorisera
seulement la composition retenue, dans la même session. Les décisions locales
réversibles peuvent être proposées sans nouveau GO générique utilisateur ; une
contradiction réelle ou nouvelle frontière doit être exposée avant code.
