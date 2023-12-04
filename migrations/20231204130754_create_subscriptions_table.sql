-- Add migration script here
-- Create Subscription Table
CREATE TABLE subscriptions (
  id uuid not null,
  PRIMARY KEY(id),
  email text not null unique,
  name text not null,
  subscribed_at timestamptz not null
);
