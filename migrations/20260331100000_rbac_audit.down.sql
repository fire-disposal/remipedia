-- ============================================
-- RBAC 系统与审计日志 回滚
-- Migration: 20260331100000_rbac_audit
-- ============================================

-- 删除触发器
DROP TRIGGER IF EXISTS update_roles_updated_at ON roles;

-- 恢复用户表
ALTER TABLE "user" DROP COLUMN IF EXISTS role_id;

-- 删除表（按依赖顺序）
DROP TABLE IF EXISTS audit_logs;
DROP TABLE IF EXISTS role_permissions;
DROP TABLE IF EXISTS permissions;
DROP TABLE IF EXISTS roles;
