# Plan de l'auteur et disposition du lead

Plan reçu de l'auteur existant, candidat `result_mvuzytnb_0000_f3igv6`, session
`ses_e75f211bfffe498yEpFr2CniSK`. Il ne s'agit ni de code ni d'une preuve runtime.
Base lue : contrat et briefing intégrés dans `5ca37b6`, consommateur `cbab06e`.

## Proposition reçue

Trois fichiers durables sous `ops/tests/k6/` : `local-private-parent.py`,
`local-private-parent.ps1`, `tests/test_local_private_parent.py`. Paramètres de
l'installation, bail et preuves restent privés hors Git. Consommateur, Child,
workflow et scénarios restent inchangés.

Le plan propose une invocation explicite de l'instance et du compte depuis un
terminal local privé. Le contexte Python reste ouvert après le retour non nul de
main. READY seul ouvre la possibilité d'une lecture ; acquittement exact
`LU <run_id>` par l'utilisateur, sans donnée du site dans le terminal de contrôle.

Observations Windows prévues : mapping Lxss unique nom/GUID/BasePath/DefaultUid/
version, SID effectif, identité dossier/VHDX, absence de reparse/substitution et
ACL, bail vivant et créneau actif. Le profil attendu sert seulement à comparer
les observations réelles.

Observations Linux prévues : linux_platform, private_path, HOME ouvert sans liens,
dev/inode/UID et modes, montage noyau ext4 lié au descripteur, refus nominal24
historique et preuves séparées, outils épinglés et identités vérifiées par phase.

Séquence en mémoire AUTHORIZE puis WRITE, IDs d'invocation/navigateur/direct,
deadline et HOME identiques. Nouvelle observation Windows pour chaque callback,
réponse fermée et bornée, fraîcheur monotone au plus 30 secondes ; défaut, lenteur
ou divergence refusés. Aucune attente humaine entre WRITE et écriture.

Retrait après LU réel ou à l'échéance UTC/monotone ; contexte quitté sur
interruption, canal fermé ou perte du bail. Exiger REMOVED et retour True, puis
joindre les enfants et libérer le seul runtime possédé devenu inutile. Aucun
acquittement artificiel ou effacement garanti après crash.

Oracles proposés : changement GUID/SID/ACL/HOME/ext4/bail/provenance, réponse
absente/tronquée/lente/étrangère, WRITE sans AUTHORIZE, mauvais LU/absence de LU,
interruption, retrait impossible, enfant Windows non joint, zéro lecture du site,
zéro donnée native publiée et non-zéro préservé. Unittest existant et diff-check,
puis préflight réel des observateurs sans session armée ni k6/Chromium.

## Réserve observée dans le plan

Les APIs consultées ne démontrent pas le lien natif Windows entre un auxiliaire
créé par interop et son PID/création/image/SID/handle de terminaison. Aucun probe
n'a été exécuté. L'interop n'est ni prouvée disponible ni déclarée impossible.

## Choix du lead, à intégrer au canon avant code

La voie **contrôleur Windows avec métadonnées sur pipes privés**, déjà proposée
par l'architecte, est retenue explicitement. Le contrôleur conserve le handle de
sa propre invocation WSL, renouvelle lui-même les observations Windows, et ne
crée aucun auxiliaire interop. Ce n'est pas un fallback automatique après échec.

Les deux scripts deviennent contrôleur Windows et compagnon Linux, sans nouvelle
autorité, RPC réseau, daemon ou superviseur général. Le compagnon compose les APIs
existantes. Les autres invariants et oracles du plan sont conservés. Le contrat
doit préciser la composition sélectionnée avant l'autorisation de code.

Routage : l'auteur existant est implementer. La composition transversale Windows/
Linux justifie une transition native vers hard-implementer dans **la même session**,
sans changement de modèle ou contournement de la politique approuvée. Ne pas lui
attribuer rétroactivement une spécialité qu'il n'avait pas.

## Vérifications effectivement faites

Plan complet récupéré et lu ; aucun code, processus WSL, outil ou site réel créé
par ce tour de plan. Les 165 tests Linux et la fermeture S-1 restent des preuves
du consommateur existant, pas du conducteur encore absent.
