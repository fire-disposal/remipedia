-- ============================================
-- Migration: 删除旧 datasheet 表
-- 方向二数据模型重构完成，datasheet 已被
-- data_streams / observations / alert_events 取代
-- ============================================

-- 1. 删除触发器与函数
DROP TRIGGER IF EXISTS trg_auto_fill_patient ON datasheet;
DROP FUNCTION IF EXISTS auto_fill_patient_id();

-- 2. 删除 datasheet 相关的索引
DROP INDEX IF EXISTS idx_datasheet_patient_time;
DROP INDEX IF EXISTS idx_datasheet_device_time;
DROP INDEX IF EXISTS idx_datasheet_subject_time;
DROP INDEX IF EXISTS idx_datasheet_type_time;
DROP INDEX IF EXISTS idx_datasheet_time;
DROP INDEX IF EXISTS idx_datasheet_source;
DROP INDEX IF EXISTS idx_datasheet_events;
DROP INDEX IF EXISTS idx_datasheet_severity;
DROP INDEX IF EXISTS idx_datasheet_status;
DROP INDEX IF EXISTS idx_datasheet_active_alerts;

-- 3. 删除约束
ALTER TABLE datasheet DROP CONSTRAINT IF EXISTS chk_data_category;
ALTER TABLE datasheet DROP CONSTRAINT IF EXISTS chk_datasheet_status;

-- 4. 删除表
DROP TABLE IF EXISTS datasheet CASCADE;
