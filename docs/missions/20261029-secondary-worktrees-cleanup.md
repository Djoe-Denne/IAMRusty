# Retrait des worktrees secondaires — 2026-10-29

## Mandat et périmètre

Demande utilisateur : retirer tous les worktrees secondaires, **après sauvegarde
vérifiée**, y compris les états non publiés. Ce nettoyage ne constitue pas une
demande de fusion automatique de toutes les branches historiques.

Les checkouts principaux d'AIForAll, de son SDK imbriqué et du clone SDK distinct
sont protégés. Les branches historiques et les modifications étrangères sont
conservées. La règle de clôture des futures missions est publiée dans le commit
`989614096110b9813ce67c47a23fad8457c2025a` ; voir le
[guide des livrables et worktrees](../guides/livrables-durables-et-worktrees.md).

## Sauvegarde et vérifications

- Inventaire exact : 86 cibles, dont 81 présentes et 5 déjà absentes.
- Sauvegardes privées hors des arbres retirés : fichiers suivis, non suivis et
  ignorés, index, patchs binaires, Gitdirs administratifs et références Git.
- 28 dépôts sauvegardés ; bundles vérifiés, miroirs restaurés et objets nécessaires
  aux HEAD et index contrôlés. Les archives sont relues et vérifiées par SHA-256.
- Une modification de deux entrées d'index a arrêté le premier préflight avant
  toute suppression. Un instantané complémentaire a conservé le nouvel état,
  sans écraser l'ancien ; les fichiers de l'archive étaient restés stables.
- La récupération est documentée dans le dossier privé de sauvegarde. Les liens
  ne sont pas suivis ; ACL et flux alternatifs NTFS ne sont pas restaurés
  automatiquement. Il ne s'agit pas d'une image forensique du disque.

Les données de développement, références privées de récupération et traces ne
sont pas publiées. Leur emplacement local et les manifestes sont consignés dans
le ledger de la mission.

## Retrait et protections

Les 86 cibles sont absentes du disque et des inventaires Git concernés, après
contrôle de stabilité des fichiers, HEAD, index et activité. Les enfants ont été
traités avant leurs parents. Un verrou Git précis a été levé dans le périmètre
autorisé. Un dossier vide retenu par Windows a été reconstruit en véritable
checkout détaché, puis retiré par l'API publique sans force.

Les SDK principaux et les fichiers étrangers protégés ont conservé leurs
empreintes. Pendant le lot, une autre session a avancé `master` et ajouté une
branche : cette avancée a été vérifiée comme descendante et préservée, pas
annulée pour obtenir un contrôle artificiellement vert. Les branches historiques
initiales sont toujours présentes ; aucune fusion aveugle, suppression globale
de branches, purge, réinitialisation ou écriture de base interne n'a été utilisée.

## État de clôture

Le retrait physique et Git est terminé. La réconciliation de l'inventaire natif
OpenCode reste en cours à cette étape ; le nettoyage complet ne sera déclaré
terminé qu'après contrôle de cet inventaire.

Vérifications de ce lot : sauvegardes, récupération en miroir, stabilité, protection
des branches et inventaires. Aucun nouveau test fonctionnel applicatif, Cargo,
Docker, Kind ou WSL. Le script AST préparé séparément reste une candidate locale,
hors du catalogue validé et hors de cette publication de nettoyage.
