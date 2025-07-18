-- Add migration script here
-- Create Subscription Tokens Table
DROP TABLE IF EXISTS subscription_tokens;

CREATE TABLE subscription_tokens(
    subscription_token TEXT NOT NULL,
    subscriber_uuid uuid NOT NULL REFERENCES subscriptions (id),
    PRIMARY KEY (subscription_token)
);