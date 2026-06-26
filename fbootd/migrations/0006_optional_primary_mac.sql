-- no-transaction
-- Make primary_mac optional and ipmi_mac mandatory. SQLite cannot alter column
-- nullability in place, so rebuild the servers table. Foreign keys must be off
-- during the rebuild so dropping the old table does not cascade-delete children.
PRAGMA foreign_keys=OFF;
BEGIN;

CREATE TABLE servers_new (
    id            TEXT PRIMARY KEY,
    primary_mac   TEXT UNIQUE,
    ipmi_mac      TEXT NOT NULL UNIQUE,
    friendly_name TEXT NOT NULL,
    hostname      TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

INSERT INTO servers_new (id, primary_mac, ipmi_mac, friendly_name, hostname, created_at, updated_at)
SELECT id, primary_mac, COALESCE(ipmi_mac, primary_mac), friendly_name, hostname, created_at, updated_at
FROM servers;

DROP TABLE servers;
ALTER TABLE servers_new RENAME TO servers;

COMMIT;
PRAGMA foreign_keys=ON;
