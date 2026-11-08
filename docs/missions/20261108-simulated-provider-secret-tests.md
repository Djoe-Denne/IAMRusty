# Fournisseur simulated — refus HTTP des mauvais credentials

## Demande

Les tests doivent utiliser le fournisseur `simulated`. Le secret transmis à ce
fournisseur doit être vérifié et une mauvaise valeur doit produire HTTP400.
Le profil ops OAuthConnect déclare déjà l'instance `simulated`, son endpoint
`/oauth/token` et l'authentification `client_secret_post`.

## Comportement et couverture

Le comportement existant de `ops/tests/support/oauth-idp-simulator.py` est
conservé : `Provider.exchange` compare réellement le client secret avec
`hmac.compare_digest`, vérifie aussi client_id, redirect, code et S256. Le handler
traduit le refus en HTTP400 avec `{"error":"invalid_grant"}`, sans credentials
dans la réponse. Un code présenté avec de mauvais bindings est consommé.

Trois tests ajoutés à `ops/tests/test_oauth_idp_simulator.py` utilisent le vrai
handler sur un serveur HTTP loopback lié au port0, pas un mock de `reply` :

1. Profil `simulated`, callback HTTPS, secret correct dans le POST : token200,
   profil200, compteurs authorize/token/profile/PKCE exacts.
2. Secret erroné puis absent :400 exact et corps générique, aucun token/profil,
   replay du code avec le bon secret toujours refusé. Un code neuf avec le secret
   correct réussit ensuite, pour discriminer un vrai refus de credential d'un
   serveur inaccessible ou d'un montage PKCE globalement invalide.
3. Faux access token sur `/user` :400, aucun profil retourné.

Le serveur et son thread sont fermés après chaque test. Ces régressions Python
valident la fixture ; elles ne réintroduisent pas un moteur E2E Python.

## Validation réelle et limites

Commande Windows native exécutée :
`python -B -m unittest discover -s ops/tests -p test_oauth_idp_simulator.py`.
Résultat : **8 tests PASS**, exit0,0.115s. Les cinq tests antérieurs sont conservés.
`git diff --check` : PASS. Aucun code de production, Rust, image ou SDK modifié.

Pas d'appel à un fournisseur réel, pas de lecture/modification OpenBao, pas de
Docker/Kind ou campagne k6 lancée. La référence attendue est une valeur dédiée
du montage test, pas une valeur réelle récupérée du coffre. La campagne complète
doit encore prouver le raccord IAM→OAuthConnect→simulated, les valeurs/configs
effectives et la distribution des secrets ; ces huit tests seuls ne prouvent
pas cette chaîne. Le refus400 est celui du fournisseur : il n'impose pas le même
statut à toutes les couches IAM/Connect qui traduisent leurs propres erreurs.
