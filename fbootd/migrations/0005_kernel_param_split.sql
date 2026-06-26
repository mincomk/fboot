ALTER TABLE bootables   ADD COLUMN cmdline TEXT;
ALTER TABLE boot_config ADD COLUMN cmdline_append TEXT;
ALTER TABLE boot_config RENAME COLUMN cmdline TO cmdline_override;
