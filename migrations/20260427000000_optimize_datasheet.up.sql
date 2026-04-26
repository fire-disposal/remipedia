-- ============================================
-- Migration: 优化 datasheet 表结构与索引
-- 1. 添加 id UUID 主键
-- 2. 删除未使用的 subject_id 列
-- 3. 添加 updated_at 列
-- 4. value_numeric 改为 DOUBLE PRECISION
-- 5. 索引优化：删除冗余，重建精华
-- ============================================

BEGIN;

-- ============================================
-- 1. datasheet 表结构优化
-- ============================================

-- 添加 id 列并填充
ALTER TABLE datasheet ADD COLUMN id UUID;
UPDATE datasheet SET id = gen_random_uuid();
ALTER TABLE datasheet ALTER COLUMN id SET NOT NULL;
ALTER TABLE datasheet ALTER COLUMN id SET DEFAULT gen_random_uuid();

-- 删除旧复合主键
ALTER TABLE datasheet DROP CONSTRAINT datasheet_pkey;

-- 保留 (time, device_id) 唯一性约束
ALTER TABLE datasheet ADD CONSTRAINT uq_datasheet_time_device UNIQUE (time, device_id);

-- 设置 id 为新主键
ALTER TABLE datasheet ADD PRIMARY KEY (id);

-- 删除未使用的 subject_id 列及相关索引
DROP INDEX IF EXISTS idx_datasheet_subject_time;
ALTER TABLE datasheet DROP COLUMN subject_id;

-- 添加 updated_at 列
ALTER TABLE datasheet ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- value_numeric 改为 DOUBLE PRECISION
ALTER TABLE datasheet ALTER COLUMN value_numeric TYPE DOUBLE PRECISION USING value_numeric::double precision;

-- ============================================
-- 2. 索引优化：删除冗余索引
-- ============================================
DROP INDEX IF EXISTS idx_datasheet_type_time;
DROP INDEX IF EXISTS idx_datasheet_severity;
DROP INDEX IF EXISTS idx_datasheet_status;

-- ============================================
-- 3. 重建优化索引（保留常用，重建事件/告警索引）
-- ============================================

-- 事件索引（替代 idx_datasheet_events，更精准的过滤条件）
DROP INDEX IF EXISTS idx_datasheet_events;
CREATE INDEX idx_datasheet_events ON datasheet(patient_id, time DESC) WHERE data_category = 'event';

-- 活跃告警索引（替代 idx_datasheet_active_alerts，包含 severity 排序）
DROP INDEX IF EXISTS idx_datasheet_active_alerts;
CREATE INDEX idx_datasheet_active_alerts ON datasheet(patient_id, severity, time DESC)
    WHERE data_category = 'event' AND status = 'active';

COMMIT;
