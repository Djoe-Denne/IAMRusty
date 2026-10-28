# Rapport 13 — arbitrage AC7 : arrêt SQS et périmètre framework

## Objectif et état

T1 a identifié une limite du framework qui affecte AC7 : arrêt borné et aucune
tâche détachée. Le contrat outbox progresse ; aucune modification du critère
ni extension framework autorisée implicitement.

## Preuves recoupées par le lead

- Le gitlink parent et le checkout rustycog pointent tous deux vers
  `6c63b65c23e26b8f83f1c6f63d656ef31c7591d0`. Pas de diff local sur sqs.rs.
- Serena, rustycog/rustycog-events/src/sqs.rs:653–711 : start spawn les pollers
  dans un Vec local de JoinHandle puis attend leurs résultats séquentiellement.
- sqs.rs:713–717 : stop positionne seulement un flag ; il ne rejoint pas les
  enfants et ne peut pas annuler un handler déjà bloqué.
- runtime/monolith/src/runtime.rs:176–204 : drain commun de cinq secondes,
  puis abort des handles parents et await de ces parents seulement.

En Rust/Tokio, abandonner un JoinHandle ne termine pas sa tâche. L'annulation
du parent peut donc détacher ses pollers ; la seule supervision du parent ne
prouve pas AC7 sur un chemin bloqué. Analyse statique recoupée, pas de test
de reproduction exécuté et pas d'affirmation de fuite observée en campagne.

## Options et recommandation

1. **Conserver AC7 intégral** : autoriser un lot préalable borné dans
   rustycog-events pour rendre le consommateur cancellation-safe, ses tests
   ciblés puis l'intégration du pin. Extension explicite au périmètre initial
   qui interdisait les changements framework ; aucun nouveau harness implicite.
2. **Conserver le framework intouché** : modifier explicitement AC7 pour
   garantir l'arrêt coopératif dans les scénarios testés, sans prétendre couvrir
   l'arrêt forcé d'un consommateur/handler bloqué. C'est une réduction réelle
   de garantie ; AC7 original ne pourrait pas être déclaré entièrement satisfait.

Recommandation : option 1 si la garantie initiale zéro tâche détachée doit être
tenue. Le coût et le blast radius du correctif restent à borner par l'architecte.
Pas de dérogation silencieuse à la qualité ni de réécriture préemptive du plan.

## Décision demandée

Autoriser l'extension minimale framework avec tests et pin, ou ratifier la
limitation coopérative d'AC7. Les autres garanties outbox ne sont pas remises
en cause par ce constat. Aucun code, test ou runtime lancé par le lead.
