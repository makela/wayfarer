-- People who get new posts by email, see src/subscribers.rs. Times are UTC,
-- as text like 2026-11-18T12:00:00Z, so they sort and compare as text.
CREATE TABLE subscribers (
    id INTEGER PRIMARY KEY,
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    -- What they told about themselves when subscribing, or ''.
    note TEXT NOT NULL DEFAULT '',
    -- The secret in the links of their emails, to confirm and unsubscribe.
    token TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    -- When they confirmed, or were imported; NULL until then.
    confirmed_at TEXT,
    -- When the last confirmation email was queued, to send at most one a day.
    confirmation_sent_at TEXT
);

-- Posts whose emails have been queued, so that each goes out once.
CREATE TABLE announced_posts (
    url TEXT PRIMARY KEY,
    announced_at TEXT NOT NULL
);

-- Emails waiting to be sent, removed once sent. kind is confirm, welcome,
-- new_subscriber (to the owner) or post (with post_url). Unsubscribing
-- removes the subscriber's waiting emails too.
CREATE TABLE outbox (
    id INTEGER PRIMARY KEY,
    subscriber_id INTEGER NOT NULL REFERENCES subscribers (id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    post_url TEXT,
    created_at TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT NOT NULL
);
CREATE INDEX outbox_next_attempt_at ON outbox (next_attempt_at);
