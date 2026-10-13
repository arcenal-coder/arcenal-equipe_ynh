CREATE SEQUENCE IF NOT EXISTS equipe_record_id;
CREATE TABLE IF NOT EXISTS equipe_records (
    kind TEXT NOT NULL CHECK (kind IN ('candidates','jobs','documents','content')),
    id BIGINT NOT NULL DEFAULT nextval('equipe_record_id'),
    data JSONB NOT NULL CHECK (jsonb_typeof(data) = 'object'),
    version BIGINT NOT NULL DEFAULT 1,
    author_uid TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (kind,id)
);
CREATE TABLE IF NOT EXISTS equipe_settings (
    singleton BOOLEAN PRIMARY KEY DEFAULT true CHECK (singleton),
    data JSONB NOT NULL DEFAULT '{}',
    author_uid TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS equipe_audit (
    id BIGSERIAL PRIMARY KEY,
    author_uid TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT NOT NULL,
    details TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS equipe_confirmations (
    nonce TEXT PRIMARY KEY,
    author_uid TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ
);
ALTER TABLE equipe_audit ADD COLUMN IF NOT EXISTS legacy_created_at TEXT;
CREATE TABLE IF NOT EXISTS equipe_imports (
    digest TEXT PRIMARY KEY,
    author_uid TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS equipe_files (
    id TEXT PRIMARY KEY,
    candidate_id BIGINT NOT NULL,
    original_name TEXT NOT NULL,
    size BIGINT NOT NULL,
    author_uid TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS equipe_records_status ON equipe_records(kind,(data->>'status'));
