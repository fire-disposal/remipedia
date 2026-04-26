-- ============================================
-- Module 去 DB 化迁移
-- 1. 删除 modules 表（Module 枚举作为唯一来源）
-- 2. role_modules 从 FK → module_code TEXT
-- ============================================

-- ============================================
-- 1. 重建 role_modules 表
-- ============================================

-- 创建临时表存储现有关联
CREATE TABLE role_modules_new (
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    module_code     VARCHAR(64) NOT NULL,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, module_code)
);

COMMENT ON TABLE role_modules_new IS '角色-模块权限关联表（module_code 替代 FK）';
COMMENT ON COLUMN role_modules_new.module_code IS '模块代码，与 Module 枚举值对应';

-- 迁移现有数据
INSERT INTO role_modules_new (role_id, module_code, granted_at)
SELECT rm.role_id, m.code, rm.granted_at
FROM role_modules rm
JOIN modules m ON rm.module_id = m.id;

-- ============================================
-- 2. 删除旧视图
-- ============================================
DROP VIEW IF EXISTS role_module_permissions;

-- ============================================
-- 3. 删除旧表
-- ============================================
DROP TRIGGER IF EXISTS update_modules_updated_at ON modules;

DROP TABLE IF EXISTS role_modules;

-- ============================================
-- 4. 新表替换旧表
-- ============================================
ALTER TABLE role_modules_new RENAME TO role_modules;

-- 索引
CREATE INDEX idx_role_modules_role ON role_modules(role_id);

-- ============================================
-- 5. 删除 modules 表
-- ============================================
DROP TABLE IF EXISTS modules;

-- ============================================
-- 6. 重建视图（基于 module_code，不再 JOIN modules 表）
-- ============================================
CREATE VIEW role_module_permissions AS
SELECT 
    r.id as role_id,
    r.name as role_name,
    r.is_system,
    rm.module_code,
    rm.granted_at
FROM roles r
LEFT JOIN role_modules rm ON r.id = rm.role_id
ORDER BY r.name, rm.module_code;

COMMENT ON VIEW role_module_permissions IS '角色模块权限视图（基于 module_code）';
