CREATE TABLE IF NOT EXISTS agents (
    guid        TEXT PRIMARY KEY NOT NULL,
    description TEXT NOT NULL,
    agent_type  TEXT NOT NULL,
    endpoint    TEXT NOT NULL,
    status      TEXT NOT NULL,
    free_cpus   INTEGER DEFAULT 0,
    free_ram    INTEGER DEFAULT 0,
    cpus        INTEGER DEFAULT 0,
    ram         INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS job_collection (
    guid        TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    description TEXT NOT NULL,
    creation_date TEXT NOT NULL,
    agent_type  TEXT NOT NULL,
    image       TEXT NOT NULL,
    cpus        INTEGER DEFAULT 0,
    ram         INTEGER DEFAULT 0,
    timeout     TEXT NOT NULL,
    target      TEXT NOT NULL,
    corpus      TEXT NOT NULL,
    status      TEXT NOT NULL,
    crash_auto_analyze BOOLEAN NOT NULL CHECK (crash_auto_analyze IN (0, 1))
);

CREATE TABLE IF NOT EXISTS jobs (
    id          INTEGER PRIMARY KEY NOT NULL,
    agent_guid  TEXT NOT NULL,
    collection_guid TEXT NOT NULL,
    idx         INTEGER DEFAULT 0,
    cpus        INTEGER DEFAULT 0,
    ram         INTEGER DEFAULT 0,
    last_msg    TEXT NOT NULL,
    status      TEXT NOT NULL,
    freed       BOOLEAN NOT NULL CHECK (freed IN (0, 1))
);

CREATE TABLE IF NOT EXISTS crashes (
    guid        TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL,
    collection_guid TEXT NOT NULL,
    analyzed    TEXT
);
