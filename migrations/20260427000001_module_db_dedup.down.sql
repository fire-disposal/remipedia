-- ============================================
-- Module 去 DB 化回滚
-- 恢复 modules 表 + FK 引用
-- ============================================

-- ============================================
-- 1. 重建 modules 表
-- ============================================
CREATE TABLE modules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    description     TEXT,
    category        TEXT NOT NULL DEFAULT 'core',
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_modules_code ON modules(code);
CREATE INDEX idx_modules_category ON modules(category);

-- 恢复模块种子数据
INSERT INTO modules (code, name, category, description) VALUES
    ('dashboard', '仪表板', 'core', '系统首页和数据概览'),
    ('patients', '患者管理', 'core', '患者信息管理和档案'),
    ('devices', '设备管理', 'core', 'IoT设备注册和管理'),
    ('bindings', '绑定关系', 'core', '设备与患者绑定管理'),
    ('data', '数据查询', 'core', '健康数据查询和导出'),
    ('users', '用户管理', 'admin', '系统用户账号管理'),
    ('roles', '角色管理', 'admin', '角色和权限配置'),
    ('audit_logs', '审计日志', 'admin', '操作日志查询'),
    ('settings', '系统设置', 'admin', '系统配置管理'),
    ('pressure_ulcer', '压疮教学', 'feature', '压力性损伤3D仿真教学');

-- ============================================
-- 2. 重建 role_modules 表（使用 FK）
-- ============================================
CREATE TABLE role_modules_new (
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    module_id       UUID NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, module_id)
);

-- 通过 module_code 恢复 FK 引用
INSERT INTO role_modules_new (role_id, module_id, granted_at)
SELECT rm.role_id, m.id, rm.granted_at
FROM role_modules rm
JOIN modules m ON rm.module_code = m.code;

-- ============================================
-- 3. 删除新表
-- ============================================
DROP TABLE IF EXISTS role_modules;

-- ============================================
-- 4. 新表替换旧表
-- ============================================
ALTER TABLE role_modules_new RENAME TO role_modules;

CREATE INDEX idx_role_modules_role ON role_modules(role_id);
CREATE INDEX idx_role_modules_module ON role_modules(module_id);

-- ============================================
-- 5. 删除旧视图，重建 JOIN modules 的视图
-- ============================================
DROP VIEW IF EXISTS role_module_permissions;

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
