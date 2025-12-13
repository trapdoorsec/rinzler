use rusqlite::{Connection, Result};
use std::fs;
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn drop(path: &Path) {
        fs::remove_file(path).unwrap();
    }
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Optimize for concurrent writes
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;  -- 64MB cache
            PRAGMA temp_store = MEMORY;
            PRAGMA foreign_keys = ON;
            ",
        )?;

        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS crawl_sessions (
    id TEXT PRIMARY KEY,
    start_time INTEGER NOT NULL,
    status TEXT NOT NULL,
    seed_urls TEXT NOT NULL  -- JSON array of starting points
);

CREATE TABLE IF NOT EXISTS maps (
    id TEXT PRIMARY KEY,
    session_id TEXT UNIQUE NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(session_id) REFERENCES crawl_sessions(id) ON DELETE CASCADE
);

-- Nodes in the graph
CREATE TABLE IF NOT EXISTS nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    map_id TEXT NOT NULL,
    url TEXT NOT NULL,
    domain TEXT NOT NULL,
    node_type TEXT NOT NULL CHECK(node_type IN ('root_host', 'endpoint', 'external_host')),

    -- Crawl metadata
    status TEXT NOT NULL DEFAULT 'pending',
    depth INTEGER NOT NULL DEFAULT 0,
    discovered_at INTEGER NOT NULL,
    last_crawled INTEGER,
    response_code INTEGER,
    response_time_ms INTEGER,
    content_hash TEXT,
    title TEXT,

    -- Graph positioning (for force-directed layout)
    position_x REAL,
    position_y REAL,

    FOREIGN KEY(map_id) REFERENCES maps(id) ON DELETE CASCADE,
    UNIQUE(map_id, url)
);

CREATE INDEX idx_nodes_map ON nodes(map_id);
CREATE INDEX idx_nodes_domain ON nodes(map_id, domain);
CREATE INDEX idx_nodes_status ON nodes(map_id, status);

-- Edges in the graph
CREATE TABLE IF NOT EXISTS edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    map_id TEXT NOT NULL,
    source_node_id INTEGER NOT NULL,
    target_node_id INTEGER NOT NULL,

    edge_type TEXT NOT NULL CHECK(edge_type IN (
        'navigation',      -- standard href link
        'reference',       -- cross-domain link
        'redirect',        -- HTTP redirect
        'form_action',     -- form submission target
        'api_call',        -- XHR/fetch endpoint
        'resource'         -- CSS/JS/image
    )),

    -- Edge metadata
    discovered_at INTEGER NOT NULL,
    link_text TEXT,           -- anchor text
    context TEXT,             -- surrounding HTML context
    http_method TEXT,         -- GET/POST/etc
    weight REAL DEFAULT 1.0,  -- for graph layout algorithms

    FOREIGN KEY(map_id) REFERENCES maps(id) ON DELETE CASCADE,
    FOREIGN KEY(source_node_id) REFERENCES nodes(id) ON DELETE CASCADE,
    FOREIGN KEY(target_node_id) REFERENCES nodes(id) ON DELETE CASCADE,
    UNIQUE(source_node_id, target_node_id, edge_type)
);

CREATE INDEX idx_edges_map ON edges(map_id);
CREATE INDEX idx_edges_source ON edges(source_node_id);
CREATE INDEX idx_edges_target ON edges(target_node_id);
            ",
        )?;
        Ok(())
    }
}
