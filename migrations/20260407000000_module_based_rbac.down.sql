-- ============================================
-- RBAC Module-Level Migration Rollback
-- ============================================

-- 删除视图
DROP VIEW IF EXISTS role_module_permissions;

-- 删除触发器
DROP TRIGGER IF EXISTS update_modules_updated_at ON modules;

-- 删除角色-模块关联表
DROP TABLE IF EXISTS role_modules;

-- 删除模块表
DROP TABLE IF EXISTS modules;

-- 恢复 patient 角色（数据需要手动处理或从备份恢复）
-- 注意：此回滚不恢复 patient 角色的用户数据
