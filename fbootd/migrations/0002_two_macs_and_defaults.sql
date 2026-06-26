ALTER TABLE servers RENAME COLUMN mac TO primary_mac;
ALTER TABLE servers ADD COLUMN ipmi_mac TEXT;
CREATE UNIQUE INDEX idx_servers_ipmi_mac ON servers(ipmi_mac) WHERE ipmi_mac IS NOT NULL;

CREATE TABLE boot_defaults (
    id                INTEGER PRIMARY KEY CHECK (id = 1),
    pxe_bootable_id   TEXT REFERENCES bootables(id) ON DELETE SET NULL,
    linux_bootable_id TEXT REFERENCES bootables(id) ON DELETE SET NULL
);
INSERT INTO boot_defaults (id) VALUES (1);
