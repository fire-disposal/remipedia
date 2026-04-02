-- ============================================
-- Ingest 原始数据归档
-- Migration: 20260402093000_ingest_raw_archive
-- ============================================

CREATE TABLE ingest_raw_data (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source          TEXT NOT NULL,
    serial_number   TEXT,
    device_type     TEXT,
    remote_addr     TEXT,
    metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
    raw_payload     BYTEA NOT NULL,
    raw_payload_text TEXT,
    status          TEXT NOT NULL DEFAULT 'stored',
    status_message  TEXT,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_ingest_raw_status CHECK (
        status IN ('stored', 'ingested', 'ignored', 'format_error', 'processing_error')
    )
);

COMMENT ON TABLE ingest_raw_data IS 'Ingest 层原始数据归档（所有进入 ingest 的数据）';
COMMENT ON COLUMN ingest_raw_data.status IS '处理状态：stored/ingested/ignored/format_error/processing_error';
COMMENT ON COLUMN ingest_raw_data.received_at IS '接收时间（进入 ingest 队列后）';
COMMENT ON COLUMN ingest_raw_data.processed_at IS '处理完成时间';

CREATE INDEX idx_ingest_raw_received_at ON ingest_raw_data(received_at DESC);
CREATE INDEX idx_ingest_raw_status_received_at ON ingest_raw_data(status, received_at DESC);
CREATE INDEX idx_ingest_raw_source_received_at ON ingest_raw_data(source, received_at DESC);
CREATE INDEX idx_ingest_raw_serial_received_at ON ingest_raw_data(serial_number, received_at DESC)
    WHERE serial_number IS NOT NULL;

CREATE TRIGGER update_ingest_raw_updated_at BEFORE UPDATE ON ingest_raw_data
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
