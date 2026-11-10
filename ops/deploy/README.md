# Déploiement local natif

```powershell
just kind-local                  # rebuild/import/lancement, données conservées
just kind-local reset-data       # destruction explicite du seul lab dédié
```

Préparer les fichiers complets du [guide opérateur](kind-minimal.md).
Le parcours utilise Bake, Kind, Helm, kubectl, OpenSSL et les CLI fournisseurs,
sans déployeur Python. Un [cycle opérateur réel](../../docs/missions/20261110-kind-local-operator-cycle.md)
a atteint 13 disponibles et HTTPS 200 après corrections/reprises explicites ;
ce n'est pas un bootstrap sans intervention ni une qualification métier globale.
Le rapport contient aussi l'arrêt/redémarrage, l'unseal Bao et la reprise SQS volatile.

L'ancien déployeur local-full, sa façade just, son chart/profils, le registre
applicatif, les outils Python de build/provenance/stockage/reprise/PKI et leurs
tests exclusifs sont supprimés. L'historique reste récupérable dans Git, pas via
une seconde entrée legacy à maintenir.

Deux scripts Python restent dans ce dossier parce qu'ils ont encore des
consommateurs actifs **hors du nouveau lanceur** :

- `local_full_target.py` : gardes de cible/baux, wrappers de déploiement
  historiques via `common.ps1`, parent k6 Windows et adaptateur Linux.
- `bootstrap_local_kind.py` : initialisation Kind/CNI gardée appelée par ces
  wrappers. Retirer ce chemin exige leur migration explicite, pas un fallback
  kubectl sans protection.

Les déclarations publiques du bootstrap Bao natif sont dans
`openbao/bootstrap.json`. Le simulateur OAuth et les catalogues nécessaires aux
tests existants ont été conservés à leurs emplacements de fixtures, sans garder
l'ancien framework pour eux. Aucun nouveau test n'est ajouté.

Le registre OCI **P4/plugins** (`p4/`, option Zot du profil natif) est distinct du
registre d'images applicatives supprimé. Les ressources/données déployées ne sont
pas touchées par ce nettoyage du dépôt.
