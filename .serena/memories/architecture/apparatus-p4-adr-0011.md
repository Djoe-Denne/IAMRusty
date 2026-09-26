# ADR-0011 — runtime plugin V1 = Pod par binding

- Jalon : P4 (complète 0009 ; mécanisme de la gate)
- Canon : `docs/adr/0011-apparatus-p4-pod-par-binding.md`
- Statut : Proposed
- Réalité : Unimplemented
- SuperSède : aucune

- Clé d’instance V1 = projet × binding (pas le digest seul) ; l’operator P4 crée / met au repos / détruit le Pod
- Même digest d’enveloppe, deux projets ⇒ deux Pods, sauf partage explicite
- Moteur = operator + Pod nu 0008 ; I/O = Lazaret ; ne SuperSède pas 0008/0009/0003/0004/0007
- `Réalité : Implemented` de 0009 ssi cet ADR est Accepted et Implemented
- Scale intelligent ou prédictif : ADR future, pas Apparatus seul (monolithe, sentinel-sync, tout service hosté). Le Pod par binding ne s'étend pas à ces services.
- Voir le fichier ADR
