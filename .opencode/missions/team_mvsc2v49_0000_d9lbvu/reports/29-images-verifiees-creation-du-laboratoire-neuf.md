# Images vérifiées, création du laboratoire neuf

Les dix images du profil OAuth ont été construites avec succès. Leurs IDs,
plateformes Linux amd64 et identités application/SDK sont vérifiés et inscrits
au registre. Les services utilisent le nouvel artefact explicite. Les deux
compilations spécialisées ont aussi réussi, sans modifier les 24 jobs.

Je passe à la création du laboratoire local-full préautorisé, uniquement s'il
est toujours absent et si ses ports restent libres. Le bootstrap existant refuse
l'adoption d'un ancien cluster, les objets d'identité inconnue et le remplacement
d'un réseau incompatible. Un bail neuf et un témoin des créations conserveront
les IDs réels, même en cas d'échec partiel.

Le cluster legacy préexistant reste intact. Aucune suppression, reprise forcée
ou opération sur un contexte protégé n'est prévue. Un cluster démarré ne serait
pas à lui seul une preuve du parcours OAuth ou du contrôle des certificats.

## Preuves

- Images : 10/10, exit 0 ; logs et IDs dans `.tmp/auth-linux-runtimes-2c14242527b1452fad17710993bd2e4d/`.
- Toutes portent app cf02c4a et SDK 02e5 ; sept consommateurs portent aussi l'ID de l'artefact vérifié.
- Ext-authz : release 40.71 s ; controller : release 33.60 s, base Rust existante conservée.
- Registre : `.tmp/docker-resources-20261029.log` ; ancien `local/build-artifacts:latest` inchangé.
