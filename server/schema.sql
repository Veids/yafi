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
    cpus        INTEGER DEFAULT 0,
    ram         INTEGER DEFAULT 0,
    timeout     TEXT NOT NULL,
    target      TEXT NOT NULL,
    corpus      TEXT NOT NULL,
    status      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS jobs (
    id          INTEGER PRIMARY KEY NOT NULL,
    agent_guid  TEXT NOT NULL,
    collection_guid TEXT NOT NULL,
    master      BOOLEAN NOT NULL CHECK (master IN (0, 1)),
    cpus        INTEGER DEFAULT 0,
    ram         INTEGER DEFAULT 0,
    last_msg    TEXT NOT NULL,
    status      TEXT NOT NULL,
    freed       BOOLEAN NOT NULL CHECK (freed IN (0, 1))
);
