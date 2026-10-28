# 13 — Correction native Windows autorisée

L'utilisateur a sélectionné **« Arrêter, corriger, relancer »**, incluant une
toolchain Go isolée si nécessaire. Les trois moniteurs possédés ont été arrêtés
avec conservation de leurs identités, locks archivés et jobs/compteurs ; les
trois watchers47328/45868/46452 ont ensuite été arrêtés gracieusement par le
contrôle natif, identité et propriété vérifiées. Proxy/cache LiteLLM conservés.
Les notifications exit1 des moniteurs correspondent à cette pause autorisée.

Préparation du correctif : source GrepAI v0.37.0 au commit
`ad90c4216282a434eeb2e28df4a1ea175a5470aa` (tag GitHub résolu).
Go absent du PATH et des emplacements usuels inspectés. Toolchain portable
officielle choisie : **go1.27.3 windows-amd64**, archive79010309octets, SHA256
`f26081f438319757f8a1b46d7d0b2c5ec7583d4038e485e0aa725cfde7885fff`,
métadonnées https://go.dev/dl/?mode=json. Installation isolée sous
`C:/Users/djden/AppData/Local/grepai-windows-fix`, sans modification du PATH global.

Avant remplacement : relever les informations de build du binaire existant,
préserver ses éventuels correctifs locaux ; ne pas remplacer aveuglément par le
tag public. Vérifier les appels Git Windows masqués, les fonctions Git et une
sonde de focus avant la relance. Aucun correctif applicatif AIForAll, aucun Git
global ni réglage Windows Terminal à modifier, aucun Docker/Kind/WSL/Cargo.
