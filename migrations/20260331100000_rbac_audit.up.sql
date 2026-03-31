-- ============================================
-- RBAC 系统与审计日志
-- Migration: 20260331100000_rbac_audit
-- ============================================

-- ============================================
-- 1. 角色表
-- ============================================
CREATE TABLE roles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL UNIQUE,
    description     TEXT,
    is_system       BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE roles IS '角色表，支持动态角色管理';
COMMENT ON COLUMN roles.is_system IS '系统内置角色，不可删除';

CREATE INDEX idx_roles_name ON roles(name);

-- ============================================
-- 2. 权限表
-- ============================================
CREATE TABLE permissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource        TEXT NOT NULL,
    action          TEXT NOT NULL,
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT uq_permission_resource_action UNIQUE (resource, action)
);

COMMENT ON TABLE permissions IS '权限表，定义资源操作';
COMMENT ON COLUMN permissions.resource IS '资源类型: user/patient/device/binding/data/system';
COMMENT ON COLUMN permissions.action IS '操作类型: create/read/update/delete/list/manage/export';

CREATE INDEX idx_permissions_resource ON permissions(resource);

-- ============================================
-- 3. 角色-权限关联表
-- ============================================
CREATE TABLE role_permissions (
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id   UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    PRIMARY KEY (role_id, permission_id)
);

CREATE INDEX idx_role_permissions_role ON role_permissions(role_id);
CREATE INDEX idx_role_permissions_perm ON role_permissions(permission_id);

-- ============================================
-- 4. 审计日志表
-- ============================================
CREATE TABLE audit_logs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID REFERENCES "user"(id) ON DELETE SET NULL,
    action          TEXT NOT NULL,           -- 操作类型
    resource        TEXT NOT NULL,           -- 资源类型
    resource_id     TEXT,                    -- 资源ID（可选）
    details         JSONB DEFAULT '{}',      -- 操作详情
    ip_address      TEXT,                    -- IP地址
    user_agent      TEXT,                    -- 用户代理
    status          TEXT NOT NULL,           -- 状态: success/failure
    error_message   TEXT,                    -- 错误信息
    duration_ms     INTEGER,                 -- 执行时长（毫秒）
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE audit_logs IS '审计日志表，记录所有重要操作';
COMMENT ON COLUMN audit_logs.action IS '操作: login/logout/create/update/delete/export/view';
COMMENT ON COLUMN audit_logs.resource IS '资源: user/patient/device/binding/data/auth/system';
COMMENT ON COLUMN audit_logs.status IS '操作状态: success/failure';

CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX idx_audit_logs_user_time ON audit_logs(user_id, created_at DESC);

-- 分区（可选，大数据量时启用）
-- CREATE INDEX idx_audit_logs_created_at_brin ON audit_logs USING BRIN(created_at);

-- ============================================
-- 5. 更新用户表，添加 role_id 外键
-- ============================================
ALTER TABLE "user" ADD COLUMN role_id UUID REFERENCES roles(id);

-- ============================================
-- 6. 初始化数据
-- ============================================

-- 插入超级管理员角色（硬编码 ID）
INSERT INTO roles (id, name, description, is_system) VALUES 
    ('00000000-0000-0000-0000-000000000001', 'super_admin', '系统超级管理员', true);

-- 插入常用角色
INSERT INTO roles (name, description) VALUES 
    ('doctor', '医生'),
    ('nurse', '护士'),
    ('caregiver', '照护者'),
    ('patient', '患者');

-- 插入所有权限
INSERT INTO permissions (resource, action, description) VALUES
    -- 用户管理
    ('user', 'create', '创建用户'),
    ('user', 'read', '查看用户'),
    ('user', 'update', '更新用户'),
    ('user', 'delete', '删除用户'),
    ('user', 'list', '列出用户'),
    ('user', 'manage', '管理用户权限'),
    -- 患者管理
    ('patient', 'create', '创建患者'),
    ('patient', 'read', '查看患者'),
    ('patient', 'update', '更新患者'),
    ('patient', 'delete', '删除患者'),
    ('patient', 'list', '列出患者'),
    -- 设备管理
    ('device', 'create', '创建设备'),
    ('device', 'read', '查看设备'),
    ('device', 'update', '更新设备'),
    ('device', 'delete', '删除设备'),
    ('device', 'list', '列出设备'),
    -- 绑定管理
    ('binding', 'create', '创建绑定'),
    ('binding', 'read', '查看绑定'),
    ('binding', 'update', '更新绑定'),
    ('binding', 'delete', '解除绑定'),
    ('binding', 'list', '列出绑定'),
    ('binding', 'switch', '切换绑定'),
    ('binding', 'history', '查看绑定历史'),
    -- 数据管理
    ('data', 'read', '查看数据'),
    ('data', 'list', '列出数据'),
    ('data', 'export', '导出数据'),
    ('data', 'aggregate', '聚合查询'),
    -- 认证
    ('auth', 'register', '用户注册'),
    ('auth', 'login', '用户登录'),
    ('auth', 'logout', '用户登出'),
    ('auth', 'refresh', '刷新令牌'),
    ('auth', 'revoke', '撤销令牌'),
    ('auth', 'manage_sessions', '管理会话'),
    -- 系统管理（仅超级管理员）
    ('system', 'manage_roles', '管理角色'),
    ('system', 'manage_permissions', '管理权限'),
    ('system', 'view_audit_logs', '查看审计日志'),
    ('system', 'manage_system_config', '管理系统配置');

-- 为角色分配权限

-- doctor: 患者、设备、绑定、数据的所有权限
INSERT INTO role_permissions (role_id, permission_id)
SELECT '00000000-0000-0000-0000-000000000001', id FROM permissions;

-- doctor: patient, device, binding, data (全部)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id 
FROM roles r, permissions p
WHERE r.name = 'doctor' 
  AND p.resource IN ('patient', 'device', 'binding', 'data', 'auth');

-- nurse: patient(read/update/list), device(read/list), binding(read/create/update/list), data(read/list/export)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id 
FROM roles r, permissions p
WHERE r.name = 'nurse' 
  AND (
    (p.resource = 'patient' AND p.action IN ('read', 'update', 'list'))
    OR (p.resource = 'device' AND p.action IN ('read', 'list'))
    OR (p.resource = 'binding' AND p.action IN ('read', 'create', 'update', 'list', 'switch'))
    OR (p.resource = 'data' AND p.action IN ('read', 'list', 'export'))
    OR (p.resource = 'auth')
  );

-- caregiver: patient(read), device(read), binding(read), data(read)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id 
FROM roles r, permissions p
WHERE r.name = 'caregiver' 
  AND (
    (p.resource IN ('patient', 'device', 'binding', 'data') AND p.action IN ('read', 'list'))
    OR (p.resource = 'auth')
  );

-- patient: data(read), patient(read 自己), binding(read 自己的)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id 
FROM roles r, permissions p
WHERE r.name = 'patient' 
  AND (
    (p.resource IN ('data', 'patient', 'binding') AND p.action IN ('read', 'list'))
    OR (p.resource = 'auth')
  );

-- ============================================
-- 7. 迁移现有用户角色数据
-- ============================================

-- 将 admin 用户设置为 super_admin
UPDATE "user" 
SET role_id = '00000000-0000-0000-0000-000000000001'
WHERE role = 'admin';

-- 将 user 用户设置为 caregiver（默认）
UPDATE "user" 
SET role_id = (SELECT id FROM roles WHERE name = 'caregiver')
WHERE role = 'user';

-- ============================================
-- 8. 删除旧的 role 列，添加 NOT NULL 约束
-- ============================================

-- 删除旧 role 列
ALTER TABLE "user" DROP COLUMN IF EXISTS role;

-- role_id 设为 NOT NULL
ALTER TABLE "user" ALTER COLUMN role_id SET NOT NULL;

-- 添加外键约束（如果还没有）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints 
        WHERE constraint_name = 'fk_user_role' 
        AND table_name = 'user'
    ) THEN
        ALTER TABLE "user" ADD CONSTRAINT fk_user_role 
        FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE RESTRICT;
    END IF;
END $$;

-- ============================================
-- 9. 触发器
-- ============================================

CREATE TRIGGER update_roles_updated_at BEFORE UPDATE ON roles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
