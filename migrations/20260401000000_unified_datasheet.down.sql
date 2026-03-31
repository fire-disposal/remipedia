-- ============================================
-- 统一数据表改造回滚
-- Migration: 20260401000000_unified_datasheet
-- ============================================

-- 删除触发器
DROP TRIGGER IF EXISTS trg_auto_fill_patient ON datasheet;
DROP FUNCTION IF EXISTS auto_fill_patient_id();

-- 删除索引
DROP INDEX IF EXISTS idx_datasheet_patient_time;
DROP INDEX IF EXISTS idx_datasheet_events;
DROP INDEX IF EXISTS idx_datasheet_severity;
DROP INDEX IF EXISTS idx_datasheet_status;
DROP INDEX IF EXISTS idx_datasheet_active_alerts;

-- 删除约束
ALTER TABLE datasheet 
    DROP CONSTRAINT IF EXISTS chk_data_category,
    DROP CONSTRAINT IF EXISTS chk_severity,
    DROP CONSTRAINT IF EXISTS chk_status;

-- 删除列
ALTER TABLE datasheet 
    DROP COLUMN IF EXISTS patient_id,
    DROP COLUMN IF EXISTS data_category,
    DROP COLUMN IF EXISTS value_numeric,
    DROP COLUMN IF EXISTS value_text,
    DROP COLUMN IF EXISTS severity,
    DROP COLUMN IF EXISTS status;

-- 删除测试数据（可选）
-- DELETE FROM datasheet WHERE source = 'system';
