# WSL refuse le nom demandé ; aucun remplacement implicite

## Objectif et résultat concret

L'instance dédiée doit être créée sur I: sans toucher aux distributions existantes.
L'image officielle Ubuntu24.04.5 est téléchargée et son SHA256 correspond exactement
aux deux sources officielles. La création native a ensuite été tentée une fois
avec le nom exact **AI for All**.

WSL rejette ce nom comme invalide. Aucun nouveau GUID, disque ou dossier de
distribution n'a été créé. ARTIST-Ubuntu reste arrêté, Docker Desktop reste actif
et étranger, et la distribution par défaut est inchangée. Il n'y a ni installation
partielle à réparer, ni nettoyage destructif à effectuer.

## Décision nécessaire et recommandation

Choisir un nom technique alternatif pour l'enregistrement WSL. Je recommande
**AIForAll**, cohérent avec le dossier `I:\WSL\AIForAll`. Cela ne change ni le disque,
la distribution Ubuntu prévue, les droits d'installation accordés ou les garanties
du diagnostic. Le nom lisible du projet reste « AI for All » dans sa documentation.

Autre possibilité : **AI-for-All**. Le changement concerne seulement le nom que
recevront les commandes WSL ; aucune des alternatives n'est prétendue déjà créée
ou testée. L'image vérifiée sera réutilisée, sans nouveau téléchargement nécessaire.

On peut aussi différer la création : aucune distribution n'est ajoutée, l'image
reste conservée sur I: et le diagnostic attend l'environnement dédié. Aucun délai
fiable n'est estimé et aucun nom ne sera essayé sans ce choix précis.

Ce n'est pas une demande de nouveau GO général : création et installations dans
l'instance restent autorisées. Le nom exact demandé a cependant été refusé, et le
contrat exclut une autre orthographe implicite.

## Annexe technique

- Image :388975696octets, SHA256
  `bb415d824822c4b878125729af451a5d18fb13d1cf5cbed9a7393ad64ac6039e`.
- Cache possédé conservé : `I:\WSL\AIForAll-acquisition-20261030-mvuqe1yi`.
- Commande : `wsl --install --from-file <image vérifiée> --name "AI for All"
  --location I:\WSL\AIForAll --no-launch --version 2` ; exit natif-1.
- Diagnostic décodé strictement UTF16LE : « Nom de distribution non valide » et
  `Wsl/Service/RegisterDistro/E_INVALIDARG`. Pas de refus de permission inventé.
- Delta Lxss nouveau0, destination Test-Pathfalse, default GUID conservé
  `{bb000c3f-6fcc-4ca1-859e-6052e02d67e0}`. États avant/après recoupés, exit0.
- Registre contemporain `.tmp/wsl-resources-20261030.log`, bail privé libéré,
  aucun runtime WSL nouveau à arrêter, aucun Docker créé/redémarré ou supprimé.
- Contrat ratifié `aca422f`, source k6 inchangée `1c0036f` ; aucun Chromium ou
  test fonctionnel exécuté, mission non clôturée.
