# IAM k6 — voie native Windows, tranche partielle

**Implémentation initiale, non qualifiée sur le lab.** Ce chemin n'appelle aucun
Python et n'installe rien. L'ancien harness et ses consommateurs restent intacts.
Contrat : [k6 IAM natif](../../../../docs/contracts/k6-iam-native-functional.md).

## Préparer / exécuter / nettoyer

Copier `config.example.json` hors Git, remplacer les chemins absolus et les SHA256
par ceux des binaires déjà présents et approuvés
(`(Get-FileHash -Algorithm SHA256 <chemin>).Hash.ToLowerInvariant()`).
k6 local observé : **2.3.0 / e088784614 / Windows amd64 / Go1.26.8** ; ce relevé ne
remplace pas la vérification du pin par le lanceur. Kubeconfig explicite, contexte
fixe `kind-aiforall-local-full`, Engine `desktop-linux` local uniquement.

La CA sélectionnée doit être **déjà approuvée dans le magasin Windows**. Aucun
import de confiance, `-k`, proxy, fallback HTTP ou `SSL_CERT_FILE` supposé fonctionner
sur Windows. Le lanceur vérifie la chaîne vers cette CA et le SAN loopback, puis
un vrai smoke k6 valide HTTPS avant les autres cas. Si le listener exige un
certificat client, le prérequis est bloqué : cette tranche n'en crée ni n'en emprunte.

```powershell
just k6-prepare iam C:\prive\iam-k6.json C:\prive\campagne-iam-001
just k6-run iam C:\prive\campagne-iam-001
just k6-clean iam C:\prive\campagne-iam-001
```

Le dossier de campagne doit être **neuf**, local et hors dépôt ; ACL limitée au
propriétaire Windows, pas de junction/symlink dans son ascendance. Utiliser les
commandes séparées permet de nettoyer même après échec. `prepare` vérifie les
outils, les trois nœuds Kind et leur port API, l'UID du namespace et le propriétaire
du Pod Envoy. Il ouvre son propre forward loopback à port éphémère vers ce Pod
(10000), jamais un port déclaré arbitrairement ou un forward préexistant. Le
processus est fermé en `finally`, comme celui de k6. Aucun arrêt du keepalive.

`run` revalide la cible et les sources, ouvre un nouveau forward et lance une
seule VU/itération. Le processus k6 ne reçoit ni options K6, proxy ou debug hérités,
ni configuration utilisateur implicite. Un résultat frais doit avoir les **12
checks uniques réussis, 12 requêtes et le marqueur de fin**, avec exit k6 nul.
Résultats expurgés : `preflight.json`, `functional.json`. Diagnostics bruts privés
dans les fichiers de phase, jamais réimprimés par le parent. `clean` retire les
entrées temporaires et invalide la campagne ; il conserve preuves et configuration.

Un verrou exclusif refuse run/clean concurrents. Après interruption brutale du
processus parent, le verrou peut rester : **pas de suppression/adoption automatique**.
Vérifier les processus locaux et les preuves avant toute reprise manuelle. Un
`finally` n'est pas une garantie après kill du parent ou panne hôte.

## Les 12 cas

- Health200/corps `OK`, JWKS public sans paramètres de clé privée, disponibilité
  d'un username éphémère (200 et `available=true`).
- Signup : email mal formé, password faible, password absent —422, sans session.
- Complétion : token malformé —400 ; refresh absent —422 ; inconnu —401.
- `/me` : Bearer absent, malformé, principal forgé sans Bearer —**403 au mesh**.
  Les401 du handler IAM direct ne sont pas l'attendu de cette route ext-authz.

Les cas POST doivent être refusés **avant écriture**. Une acceptation inattendue
échoue immédiatement ; elle ne sera pas effacée par un SQL de secours. Dans ce
cas, `cleanup.json` ne prouve pas l'absence d'effets métier : conserver le run_id
et son email `k6_<run_id>@example.invalid` pour investigation ciblée. Le cleanup
ne supprime aucune donnée métier et ne prétend pas réparer un bug du produit.

## Explicitement non couvert

Pas de comptes A/B créés, login positif, rotation/rejeu de refresh valide,
vérification email, isolation entre utilisateurs/organisations, JWT expiré valide,
OAuth, mutation SAN ou clés. **Ce n'est pas une suite IAM complète ni un audit de
sécurité.** Le parcours positif exige un inverse causal sûr : l'inscription émet
`UserSignedUp`, déjà consommable par Telegraph ; supprimer les comptes et l'outbox
n'annule ni messages ni notifications en aval. Aucun accord de rétention de ces
effets n'a été présumé. Pas de fixture SQL inutile dans la tranche active.

Dernier constat live antérieur : API Kubernetes inaccessible, malgré trois nœuds
running, et port API Docker absent. Aucun nouveau run live, démarrage ou réparation
implicite effectué pour cette livraison. `prepare` échouera tant que ce prérequis
ou la confiance TLS n'est pas satisfait ; un dry-run/inspect n'est pas un PASS live.

Sources TLS : [Go Windows system verifier](https://raw.githubusercontent.com/golang/go/master/src/crypto/x509/root_windows.go),
[k6 TLS](https://grafana.com/docs/k6/latest/using-k6/protocols/ssl-tls/).
