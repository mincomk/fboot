CREATE TABLE servers (
    id            TEXT PRIMARY KEY,
    mac           TEXT NOT NULL UNIQUE,
    friendly_name TEXT NOT NULL,
    hostname      TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE server_metadata (
    server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    key       TEXT NOT NULL,
    value     TEXT NOT NULL,
    PRIMARY KEY (server_id, key)
);

CREATE TABLE server_ipmi (
    server_id TEXT PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    host      TEXT,
    username  TEXT,
    password  TEXT,
    cipher    INTEGER
);

CREATE TABLE bootables (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL
);

CREATE TABLE bootable_files (
    bootable_id TEXT NOT NULL REFERENCES bootables(id) ON DELETE CASCADE,
    role        TEXT NOT NULL,
    source      TEXT NOT NULL,
    location    TEXT NOT NULL,
    PRIMARY KEY (bootable_id, role)
);

CREATE TABLE bootable_metadata (
    bootable_id TEXT NOT NULL REFERENCES bootables(id) ON DELETE CASCADE,
    key         TEXT NOT NULL,
    value       TEXT NOT NULL,
    PRIMARY KEY (bootable_id, key)
);

CREATE TABLE boot_config (
    server_id         TEXT PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    boot_pxe          INTEGER NOT NULL DEFAULT 0,
    pxe_bootable_id   TEXT REFERENCES bootables(id) ON DELETE SET NULL,
    linux_bootable_id TEXT REFERENCES bootables(id) ON DELETE SET NULL,
    cmdline           TEXT
);

CREATE TABLE stats (
    server_id    TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    ts           TEXT NOT NULL,
    power_status TEXT NOT NULL,
    power_w      REAL,
    cpu_temp_c   REAL,
    PRIMARY KEY (server_id, ts)
);

CREATE INDEX idx_stats_server_ts ON stats(server_id, ts DESC);
