-- ============================================
-- RBAC Module-Level Migration
-- 简化权限控制：从细粒度 resource:action 降级为 module 级别
-- ============================================

-- ============================================
-- 1. 模块定义表
-- ============================================
CREATE TABLE modules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    description     TEXT,
    category        TEXT NOT NULL DEFAULT 'core',  -- core, admin, feature
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE modules IS '系统模块定义表';
COMMENT ON COLUMN modules.code IS '模块代码，用于权限检查';
COMMENT ON COLUMN modules.category IS '模块分类: core(核心功能), admin(管理功能), feature(特色功能)';

CREATE INDEX idx_modules_code ON modules(code);
CREATE INDEX idx_modules_category ON modules(category);

-- ============================================
-- 2. 角色-模块权限关联表
-- ============================================
CREATE TABLE role_modules (
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    module_id       UUID NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    PRIMARY KEY (role_id, module_id)
);

COMMENT ON TABLE role_modules IS '角色-模块权限关联表，二级权限控制';

CREATE INDEX idx_role_modules_role ON role_modules(role_id);
CREATE INDEX idx_role_modules_module ON role_modules(module_id);

-- ============================================
-- 3. 初始化模块数据
-- ============================================
INSERT INTO modules (code, name, category, description) VALUES
    -- 核心功能模块
    ('dashboard', '仪表板', 'core', '系统首页和数据概览'),
    ('patients', '患者管理', 'core', '患者信息管理和档案'),
    ('devices', '设备管理', 'core', 'IoT设备注册和管理'),
    ('bindings', '绑定关系', 'core', '设备与患者绑定管理'),
    ('data', '数据查询', 'core', '健康数据查询和导出'),
    
    -- 管理功能模块
    ('users', '用户管理', 'admin', '系统用户账号管理'),
    ('roles', '角色管理', 'admin', '角色和权限配置'),
    ('audit_logs', '审计日志', 'admin', '操作日志查询'),
    ('settings', '系统设置', 'admin', '系统配置管理'),
    
    -- 特色功能模块
    ('pressure_ulcer', '压疮教学', 'feature', '压力性损伤3D仿真教学');

-- ============================================
-- 4. 为现有角色分配模块权限
-- ============================================

-- admin (super_admin): 所有模块（通配权限通过 is_system=true 实现，此处也插入以便查询）
INSERT INTO role_modules (role_id, module_id)
SELECT r.id, m.id 
FROM roles r, modules m
WHERE r.name = 'super_admin';

-- doctor: 核心模块
INSERT INTO role_modules (role_id, module_id)
SELECT r.id, m.id 
FROM roles r, modules m
WHERE r.name = 'doctor' 
  AND m.category IN ('core', 'feature');

-- nurse: 核心模块（与 doctor 相同，未来可细分）
INSERT INTO role_modules (role_id, module_id)
SELECT r.id, m.id 
FROM roles r, modules m
WHERE r.name = 'nurse' 
  AND m.category IN ('core', 'feature');

-- caregiver: 核心模块（不含设备管理）
INSERT INTO role_modules (role_id, module_id)
SELECT r.id, m.id 
FROM roles r, modules m
WHERE r.name = 'caregiver' 
  AND m.code IN ('dashboard', 'patients', 'bindings', 'data', 'pressure_ulcer');

-- patient 角色将被删除，不分配模块

-- ============================================
-- 5. 清理 patient 角色及相关数据
-- ============================================

-- 删除 patient 角色的权限关联
DELETE FROM role_permissions 
WHERE role_id IN (SELECT id FROM roles WHERE name = 'patient');

-- 将现有 patient 角色用户迁移到 caregiver 角色
UPDATE "user" 
SET role_id = (SELECT id FROM roles WHERE name = 'caregiver')
WHERE role_id IN (SELECT id FROM roles WHERE name = 'patient');

-- 删除 patient 角色
DELETE FROM roles WHERE name = 'patient';

-- ============================================
-- 6. 更新角色表（确保有系统管理员）
-- ============================================

-- 确保 super_admin 是系统角色
UPDATE roles 
SET is_system = true, 
    description = '系统管理员（拥有所有模块权限）'
WHERE name = 'super_admin';

-- 如果有多个系统角色，确保至少有一个
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM roles WHERE is_system = true) THEN
        UPDATE roles
        SET is_system = true
        WHERE id = (
            SELECT id FROM roles WHERE name = 'super_admin' OR name = 'admin' LIMIT 1
        );
    END IF;
END $$;

-- ============================================
-- 7. 添加触发器（保持 updated_at 同步）
-- ============================================

CREATE TRIGGER update_modules_updated_at BEFORE UPDATE ON modules
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================
-- 8. 视图：角色完整权限（便于查询）
-- ============================================

CREATE VIEW role_module_permissions AS
SELECT 
    r.id as role_id,
    r.name as role_name,
    r.is_system,
    m.id as module_id,
    m.code as module_code,
    m.name as module_name,
    m.category as module_category
FROM roles r
LEFT JOIN role_modules rm ON r.id = rm.role_id
LEFT JOIN modules m ON rm.module_id = m.id
ORDER BY r.name, m.category, m.code;

COMMENT ON VIEW role_module_permissions IS '角色模块权限完整视图';
