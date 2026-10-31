# Compilation Linux validée, images de service en préparation

Le premier build Linux a réussi depuis le contexte propre : compilation
`release --locked`, puis export des artefacts. L'image retournée est bien
Linux amd64 et porte les identités exactes de l'application et du SDK.

Son ID est enregistré au registre de ressources. L'ancien artefact inconnu et
les conteneurs préexistants sont inchangés. Aucun service applicatif ni cluster
n'a été démarré ; ce succès de compilation n'est pas une preuve Envoy ou navigateur.

Je poursuis avec les dix images du profil local-full, une à la fois. Les services
recevront explicitement le nouvel artefact, sans utiliser le tag historique.
Les deux builders spécialisés gardent leurs 24 jobs ; leur compatibilité réelle
reste à observer. Un échec arrêtera le lot sans nettoyage destructif.

## Preuves

- Build principal : exit 0, 116.6 s ; compilation release terminée en 1 min 31 s.
- Application cf02c4a, SDK 02e5 ; contexte transféré annoncé par BuildKit : 14.52 MB.
- Image : `sha256:aa7c846a04953b5961f9d4a2419b75c8c52e64b5beb455b12f399265c3054c92`.
- Preuves : `.tmp/auth-linux-build-68ae9525af944262a082368e59349451/`.
- Registre : `.tmp/docker-resources-20261029.log` ; image requise par la livraison, caches opaques non adoptés.
