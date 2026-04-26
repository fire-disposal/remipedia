-- ============================================
-- Down Migration: 恢复 datasheet 表
-- ============================================

-- 1. 重建 datasheet 表（最终结构）
CREATE TABLE datasheet (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    time TIMESTAMPTZ NOT NULL,
    device_id UUID REFERENCES device(id) ON DELETE SET NULL,
    patient_id UUID REFERENCES patient(id) ON DELETE SET NULL,
    data_type TEXT NOT NULL,
    data_category TEXT NOT NULL DEFAULT 'metric',
    value_numeric DOUBLE PRECISION,
    value_text TEXT,
    severity TEXT,
    status TEXT,
    payload JSONB DEFAULT '{}',
    source TEXT NOT NULL,
    ingested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_datasheet_time_device UNIQUE (time, device_id),
    CONSTRAINT chk_data_category CHECK (data_category IN ('metric', 'event')),
    CONSTRAINT chk_datasheet_status CHECK (status IN ('active', 'acknowledged', 'resolved'))
);

-- 2. 重建索引
CREATE INDEX idx_datasheet_patient_time ON datasheet(patient_id, time DESC) WHERE patient_id IS NOT NULL;
CREATE INDEX idx_datasheet_device_time ON datasheet(device_id, time DESC);
CREATE INDEX idx_datasheet_type_time ON datasheet(data_type, time DESC);
CREATE INDEX idx_datasheet_time ON datasheet(time DESC);
CREATE INDEX idx_datasheet_source ON datasheet(source);
CREATE INDEX idx_datasheet_events ON datasheet(patient_id, time DESC) WHERE data_category = 'event';
CREATE INDEX idx_datasheet_active_alerts ON datasheet(patient_id, severity, time DESC)
    WHERE data_category = 'event' AND status = 'active';

-- 3. 重建自动填充触发器
CREATE OR REPLACE FUNCTION auto_fill_patient_id()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.patient_id IS NULL AND NEW.device_id IS NOT NULL THEN
        SELECT b.patient_id INTO NEW.patient_id
        FROM binding b
        WHERE b.device_id = NEW.device_id
          AND b.deleted_at IS NULL
          AND (b.ended_at IS NULL OR b.ended_at > NEW.time)
        ORDER BY b.created_at DESC
        LIMIT 1;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_auto_fill_patient
    BEFORE INSERT ON datasheet
    FOR EACH ROW
    EXECUTE FUNCTION auto_fill_patient_id();

COMMENT ON TABLE datasheet IS '时间序列数据表（已废弃，由 data_streams/observations/alert_events 替代）';
