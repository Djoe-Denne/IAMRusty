Décision utilisateur 2026-09-22 : pour que T11 (`t11_plugin_exec_cannot_reach_transit_or_system`) soit une vraie preuve d’isolation plugin↔système, un CNI qui enforce NetworkPolicy (Calico ou Cilium) est une NÉCESSITÉ, pas une option. kindnet (disableDefaultCNI: false) n’applique pas les NP.

Ne pas flipper ADR-0008 en Implemented même après Calico (Transit -dev, Adm-B restent). Partial TENU.
ADR-0007 Implemented TENU.

Briefing réconciliation : `.cursor/review-briefings/20260922T0726Z-reconcile-adr-0007-0008-c465fb7.md`