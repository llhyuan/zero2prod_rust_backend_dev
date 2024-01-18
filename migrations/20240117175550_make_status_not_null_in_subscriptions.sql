-- Add migration script here
-- the BEGIN and COMMIT marks a unit of atomic database oprations
-- It makes sure that either all operations in between are carried out or none. 
-- ensuring data integraty. 
BEGIN;
    -- Backfill `status` for historical entries
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;
    -- Make `status` mandatory
    ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;

