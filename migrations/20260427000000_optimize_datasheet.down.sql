-- ============================================
-- Down Migration: 还原 datasheet 表结构
-- ============================================

BEGIN;

-- 恢复索引
DROP INDEX IF EXISTS idx_datasheet_events;
DROP INDEX IF EXISTS idx_datasheet_active_alerts;
CREATE INDEX idx_datasheet_events ON datasheet(patient_id, time DESC) WHERE data_category = 'event';
CREATE INDEX idx_datasheet_active_alerts ON datasheet(patient_id, time DESC) WHERE data_category = 'event' AND status = 'active';
CREATE INDEX IF NOT EXISTS idx_datasheet_type_time ON datasheet(data_type, time DESC);
CREATE INDEX IF NOT EXISTS idx_datasheet_severity ON datasheet(severity) WHERE severity IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_datasheet_status ON datasheet(status) WHERE status IS NOT NULL;

-- 恢复 subject_id
ALTER TABLE datasheet ADD COLUMN subject_id UUID REFERENCES patient(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_datasheet_subject_time ON datasheet(subject_id, time DESC);

-- 恢复 value_numeric 类型
ALTER TABLE datasheet ALTER COLUMN value_numeric TYPE DECIMAL(10,4) USING value_numeric::numeric(10,4);

-- 删除 updated_at
ALTER TABLE datasheet DROP COLUMN updated_at;

-- 恢复复合主键
ALTER TABLE datasheet DROP CONSTRAINT uq_datasheet_time_device;
ALTER TABLE datasheet DROP CONSTRAINT datasheet_pkey;
ALTER TABLE datasheet ADD PRIMARY KEY (time, device_id);

-- 删除 id 列
ALTER TABLE datasheet DROP COLUMN id;

COMMIT;
