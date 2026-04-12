-- Migration 013: Clear pre-seeded sample flower sorts.
-- The 60 sorts inserted in migration 004 all follow the 'fs-NNN' ID pattern.
-- Real sorts created by the user are assigned random UUIDs and are unaffected.
DELETE FROM pack_assignments       WHERE sort_id LIKE 'fs-%';
DELETE FROM packaging_log          WHERE sort_id LIKE 'fs-%';
DELETE FROM greenhouse_harvest_log WHERE sort_id LIKE 'fs-%';
DELETE FROM flower_sorts           WHERE id      LIKE 'fs-%';
