-- Create manifest_entries table for individual manifest entries
CREATE TABLE IF NOT EXISTS manifest_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    data JSONB
);

-- Create manifests table to track versioned manifest configurations
CREATE TABLE IF NOT EXISTS manifests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    notes TEXT
);

-- Create manifest_to_entry junction table to link manifests to their entries
CREATE TABLE IF NOT EXISTS manifest_to_entry (
    manifest_id INTEGER NOT NULL REFERENCES manifests(id),
    entry_id INTEGER NOT NULL REFERENCES manifest_entries(id),
    PRIMARY KEY (manifest_id, entry_id)
);

CREATE TABLE IF NOT EXISTS datasheets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    manifest_id TEXT REFERENCES manifests(uuid),
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);


-- Create datasheet_entries table
CREATE TABLE IF NOT EXISTS datasheet_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    manifest_entry_id TEXT REFERENCES manifest_entries(uuid),
    data JSONB
);

-- Create datasheet_to_entry junction table to link manifests to their entries
CREATE TABLE IF NOT EXISTS datasheet_to_entry (
    datasheet_id INTEGER NOT NULL REFERENCES datasheets(id),
    entry_id INTEGER REFERENCES datasheet_entries(id),
    PRIMARY KEY (datasheet_id, entry_id)
);


