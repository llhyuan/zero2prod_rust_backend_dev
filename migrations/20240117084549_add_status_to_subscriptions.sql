-- Add migration script here
-- Alter table Subscriptions, to add new colums
ALTER TABLE subscriptions
  ADD COLUMN status TEXT null;
