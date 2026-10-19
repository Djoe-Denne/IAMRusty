-- DEPRECATED — ne pas appliquer. Hors gold 0605 / 0007 (pas de seed SQL).
-- digest 0.1.0 est faux (n'est pas le pin descripteur 0002).
-- Conservé hors nominal ; just prove-gold n'utilise pas ce fichier.
-- J3 demo leftover: T4 declared ∩ T5 consents for lazaret-enroll-identity binding.
-- Binding UUID matches ConfigMap lazaret-enroll-identity (ns apparatus-plugins).
-- digest = identity.release (job RELEASE=0.1.0). grant_revision=1 on binding and consents.
-- component pending → active: evaluate_grant denies ComponentInactive otherwise.

BEGIN;

UPDATE project_components
SET status = 'active'
WHERE id = '57756e60-498e-4d0e-bd30-a35888afa660';

UPDATE apparatus_bindings
SET
    declared_capabilities = '["storage.kv.read", "storage.kv.write"]'::jsonb,
    digest = '0.1.0',
    grant_revision = 1
WHERE component_id = '57756e60-498e-4d0e-bd30-a35888afa660';

INSERT INTO apparatus_capability_consents (component_id, capability, status, grant_revision)
VALUES
    ('57756e60-498e-4d0e-bd30-a35888afa660', 'storage.kv.read', 'consented', 1),
    ('57756e60-498e-4d0e-bd30-a35888afa660', 'storage.kv.write', 'consented', 1)
ON CONFLICT (component_id, capability) DO UPDATE
SET status = EXCLUDED.status,
    grant_revision = EXCLUDED.grant_revision;

COMMIT;
