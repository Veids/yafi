CREATE TABLE IF NOT EXISTS agents (
    guid        TEXT PRIMARY KEY NOT NULL,
    description TEXT NOT NULL,
    agent_type        TEXT NOT NULL,
    endpoint    TEXT NOT NULL,
    status      TEXT NOT NULL
);
