CREATE TABLE IF NOT EXISTS admins (
    user_id INTEGER PRIMARY KEY,
    name TEXT,
    added_by INTEGER,
    added_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(added_by) REFERENCES admins(user_id) ON DELETE CASCADE
);

INSERT INTO admins (user_id, name, added_by) VALUES (640129894, 'justlelis', NULL)
ON CONFLICT(user_id) DO NOTHING;

CREATE TABLE IF NOT EXISTS whitelisted_groups (
    group_id INTEGER PRIMARY KEY,
    group_name TEXT,

    added_by INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(added_by) REFERENCES admins(user_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS whitelisted_threads (
    thread_id INTEGER,
    group_id INTEGER,

    group_name TEXT,
    thread_name TEXT,

    added_by INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(thread_id, group_id),
    FOREIGN KEY(group_id) REFERENCES whitelisted_groups(group_id) ON DELETE CASCADE
    FOREIGN KEY(added_by) REFERENCES admins(user_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS become_admin_requests (
    request_id TEXT PRIMARY KEY,
    user_id INTEGER,
    user_name TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    pending BOOLEAN DEFAULT TRUE,
    accepted BOOLEAN
);
