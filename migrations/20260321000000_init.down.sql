-- ============================================
-- IoT Health Platform 数据库回滚
-- Migration: 20260321000000_init
-- ============================================

-- 删除触发器
DROP TRIGGER IF EXISTS update_device_updated_at ON device;
DROP TRIGGER IF EXISTS update_patient_profile_updated_at ON patient_profile;
DROP TRIGGER IF EXISTS update_patient_updated_at ON patient;
DROP TRIGGER IF EXISTS update_user_updated_at ON "user";

-- 删除触发器函数
DROP FUNCTION IF EXISTS update_updated_at_column();

-- 删除表（按依赖顺序倒序）
DROP TABLE IF EXISTS datasheet;
DROP TABLE IF EXISTS binding;
DROP TABLE IF EXISTS device;
DROP TABLE IF EXISTS user_patient_binding;
DROP TABLE IF EXISTS patient_profile;
DROP TABLE IF EXISTS patient;
DROP TABLE IF EXISTS "user";