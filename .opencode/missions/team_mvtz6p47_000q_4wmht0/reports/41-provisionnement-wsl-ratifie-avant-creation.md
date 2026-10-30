# Provisionnement de l'instance WSL ratifié avant création

## Décision et périmètre

La demande d'une instance **AI for All sur I:** est traduite dans le contrat puis
ratifiée avant création. Le lead utilise Ubuntu24.04.5 amd64 depuis son image
officielle versionnée, avec SHA256 vérifié avant installation. L'emplacement prévu
est `I:\WSL\AIForAll`, sans collision observée au préflight.

L'administration pour installer les outils dans cette instance neuve est autorisée.
Les validations k6/Chromium utiliseront un compte de travail non-root. Aucune
modification de l'ancienne distribution, du défaut WSL, des garanties de sandbox
ou des binaires épinglés n'est admise. Aucun nouveau GO de routine n'est requis.

## Suite bornée

Acquérir l'image vérifiée, installer depuis le fichier local avec nom/emplacement
explicites et sans lancement automatique, puis relever le GUID réel et le disque
créés. Un refus du nom demandé n'autorisera pas une autre orthographe implicite.
Créer ensuite le compte de travail et vérifier l'identité de l'hôte avant outils
et tests. La distribution et ses données resteront conservées ; son runtime reste
à la demande, avec bail et arrêt ciblé quand inutile.

Le relevé source privé reste une proposition séparée : aucune collecte ni preuve
de qualification ne découle de cet accord. Une nouvelle instance n'est pas une
reproduction démontrée du runner GitHub. Aucun outil Rust ou Docker n'est installé
spontanément ; les builds/tests Rust de développement restent Windows.

## Preuves

- Candidat architecte `result_mvuqe1yi_0002_ar0a9l`, session conservée :70lignes
  ajoutées, seul nouveau contrat modifié, diff-check auteur exit0. Récupération
  complète byte-identique puis sauvegarde avant ratification.
- Sauvegarde originale :
  `.tmp/k6-dedicated-wsl-contract-proposed-93bf814808684c0f93cd8e6eba86f29b.md`.
- Image officielle attendue :388975696octets ; SHA256
  `bb415d824822c4b878125729af451a5d18fb13d1cf5cbed9a7393ad64ac6039e`,
  concordant entre catalogue WSL et SHA256SUMS Ubuntu. Octets pas encore acquis.
- Registre et bail propres seront écrits avant création ; aucun ancien bail adopté.
  Aucun dossier/instance créé, téléchargement ou installation acquis à la rédaction.
