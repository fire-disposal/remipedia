-- ============================================
-- IoT Health Platform 最终状态合并初始化脚本
-- 已去除所有中间迁移的无用过程，直接构建最终表结构与基础数据
-- ============================================

BEGIN;

-- ============================================
-- 1. 更新时间触发器基础函数
-- ============================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS }
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
} language 'plpgsql';

-- ============================================
-- 2. RBAC系统：角色与模块权限
-- ============================================

-- 角色表
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

CREATE TRIGGER update_roles_updated_at BEFORE UPDATE ON roles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 细粒度权限表（历史保留，现已降级为模块级）
CREATE TABLE permissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    resource        TEXT NOT NULL,
    action          TEXT NOT NULL,
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_permission_resource_action UNIQUE (resource, action)
);
CREATE INDEX idx_permissions_resource ON permissions(resource);

CREATE TABLE role_permissions (
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id   UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, permission_id)
);
CREATE INDEX idx_role_permissions_role ON role_permissions(role_id);
CREATE INDEX idx_role_permissions_perm ON role_permissions(permission_id);

-- 模块表（当前使用的权限粒度）
CREATE TABLE modules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    description     TEXT,
    category        TEXT NOT NULL DEFAULT 'core',  
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE modules IS '系统模块定义表';
COMMENT ON COLUMN modules.category IS '模块分类: core(核心功能), admin(管理功能), feature(特色功能)';
CREATE INDEX idx_modules_code ON modules(code);
CREATE INDEX idx_modules_category ON modules(category);

CREATE TRIGGER update_modules_updated_at BEFORE UPDATE ON modules
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 角色-模块映射表
CREATE TABLE role_modules (
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    module_id       UUID NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (role_id, module_id)
);
CREATE INDEX idx_role_modules_role ON role_modules(role_id);
CREATE INDEX idx_role_modules_module ON role_modules(module_id);

-- ============================================
-- 3. 用户与认证体系
-- ============================================

-- 用户表
CREATE TABLE "user" (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE RESTRICT,
    phone           TEXT UNIQUE,
    email           TEXT UNIQUE,
    avatar_url      TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    last_login_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE "user" IS '系统用户表';
CREATE INDEX idx_user_username ON "user"(username);
CREATE INDEX idx_user_phone ON "user"(phone);
CREATE INDEX idx_user_email ON "user"(email);
CREATE INDEX idx_user_status ON "user"(status);

CREATE TRIGGER update_user_updated_at BEFORE UPDATE ON "user"
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 刷新令牌表
CREATE TABLE refresh_tokens (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL UNIQUE,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at      TIMESTAMPTZ
);
CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_hash ON refresh_tokens(token_hash);
CREATE INDEX idx_refresh_tokens_active ON refresh_tokens(user_id) WHERE revoked_at IS NULL;

-- 审计日志表
CREATE TABLE audit_logs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID REFERENCES "user"(id) ON DELETE SET NULL,
    action          TEXT NOT NULL,           
    resource        TEXT NOT NULL,           
    resource_id     TEXT,                    
    details         JSONB DEFAULT '{}',      
    ip_address      TEXT,                    
    user_agent      TEXT,                    
    status          TEXT NOT NULL,           
    error_message   TEXT,                    
    duration_ms     INTEGER,                 
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at DESC);

-- ============================================
-- 4. 患者与设备体系
-- ============================================

-- 患者表
CREATE TABLE patient (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    external_id     TEXT UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_patient_external_id ON patient(external_id);

CREATE TRIGGER update_patient_updated_at BEFORE UPDATE ON patient
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 患者档案表
CREATE TABLE patient_profile (
    patient_id      UUID PRIMARY KEY REFERENCES patient(id) ON DELETE CASCADE,
    date_of_birth   DATE,
    gender          TEXT,
    blood_type      TEXT,
    contact_phone   TEXT,
    address         TEXT,
    emergency_contact TEXT,
    emergency_phone TEXT,
    medical_id      TEXT UNIQUE,
    allergies       JSONB DEFAULT '[]',
    medical_history JSONB DEFAULT '[]',
    notes           TEXT,
    tags            JSONB DEFAULT '[]',
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_patient_profile_medical_id ON patient_profile(medical_id);
CREATE INDEX idx_patient_profile_tags ON patient_profile USING GIN(tags);

CREATE TRIGGER update_patient_profile_updated_at BEFORE UPDATE ON patient_profile
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 用户-患者绑定表
CREATE TABLE user_patient_binding (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    patient_id      UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    relation        TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_user_patient UNIQUE (user_id, patient_id)
);
CREATE INDEX idx_user_patient_binding_user ON user_patient_binding(user_id);
CREATE INDEX idx_user_patient_binding_patient ON user_patient_binding(patient_id);

-- 设备表
CREATE TABLE device (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    serial_number   TEXT NOT NULL UNIQUE,
    device_type     TEXT NOT NULL,
    firmware_version TEXT,
    status          TEXT NOT NULL DEFAULT 'inactive',
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_device_serial ON device(serial_number);
CREATE INDEX idx_device_status ON device(status);
CREATE INDEX idx_device_type ON device(device_type);

CREATE TRIGGER update_device_updated_at BEFORE UPDATE ON device
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 设备-患者绑定表
CREATE TABLE binding (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id       UUID NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    patient_id      UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at        TIMESTAMPTZ,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX idx_binding_unique_active ON binding(device_id) WHERE ended_at IS NULL;
CREATE INDEX idx_binding_device ON binding(device_id);
CREATE INDEX idx_binding_patient ON binding(patient_id);
CREATE INDEX idx_binding_time ON binding(started_at, ended_at);

-- ============================================
-- 5. 数据采集体系 (Ingest & Datasheet)
-- ============================================

-- 原始数据层
CREATE TABLE ingest_raw_data (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source          TEXT NOT NULL,
    serial_number   TEXT,
    device_type     TEXT,
    remote_addr     TEXT,
    metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
    raw_payload     BYTEA NOT NULL,
    raw_payload_text TEXT,
    status          TEXT NOT NULL DEFAULT 'stored',
    status_message  TEXT,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_ingest_raw_status CHECK (status IN ('stored', 'ingested', 'ignored', 'format_error', 'processing_error'))
);
CREATE INDEX idx_ingest_raw_received_at ON ingest_raw_data(received_at DESC);
CREATE INDEX idx_ingest_raw_status_received_at ON ingest_raw_data(status, received_at DESC);
CREATE INDEX idx_ingest_raw_source_received_at ON ingest_raw_data(source, received_at DESC);
CREATE INDEX idx_ingest_raw_serial_received_at ON ingest_raw_data(serial_number, received_at DESC) WHERE serial_number IS NOT NULL;

CREATE TRIGGER update_ingest_raw_updated_at BEFORE UPDATE ON ingest_raw_data
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 统一时间序列数据表
CREATE TABLE datasheet (
    time            TIMESTAMPTZ NOT NULL,
    device_id       UUID NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    subject_id      UUID REFERENCES patient(id) ON DELETE SET NULL,
    patient_id      UUID REFERENCES patient(id) ON DELETE SET NULL,
    data_type       TEXT NOT NULL,
    data_category   TEXT NOT NULL DEFAULT 'metric',
    payload         JSONB NOT NULL,
    value_numeric   DECIMAL(10,4),
    value_text      TEXT,
    severity        TEXT,
    status          TEXT DEFAULT NULL,
    source          TEXT NOT NULL DEFAULT 'mqtt',
    ingested_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    PRIMARY KEY (time, device_id),
    CONSTRAINT chk_data_category CHECK (data_category IN ('metric', 'event')),
    CONSTRAINT chk_severity CHECK (severity IS NULL OR severity IN ('info', 'warning', 'alert')),
    CONSTRAINT chk_status CHECK (status IS NULL OR status IN ('active', 'acknowledged', 'resolved'))
);

CREATE INDEX idx_datasheet_device_time ON datasheet(device_id, time DESC);
CREATE INDEX idx_datasheet_subject_time ON datasheet(subject_id, time DESC);
CREATE INDEX idx_datasheet_patient_time ON datasheet(patient_id, time DESC) WHERE patient_id IS NOT NULL;
CREATE INDEX idx_datasheet_type_time ON datasheet(data_type, time DESC);
CREATE INDEX idx_datasheet_events ON datasheet(patient_id, time DESC) WHERE data_category = 'event';
CREATE INDEX idx_datasheet_severity ON datasheet(severity) WHERE severity IS NOT NULL;
CREATE INDEX idx_datasheet_status ON datasheet(status) WHERE status IS NOT NULL;
CREATE INDEX idx_datasheet_active_alerts ON datasheet(patient_id, time DESC) WHERE data_category = 'event' AND status = 'active';

CREATE OR REPLACE FUNCTION auto_fill_patient_id()
RETURNS TRIGGER AS }
BEGIN
    IF NEW.patient_id IS NULL AND NEW.device_id IS NOT NULL THEN
        SELECT patient_id INTO NEW.patient_id FROM binding
        WHERE device_id = NEW.device_id AND ended_at IS NULL
        ORDER BY started_at DESC LIMIT 1;
    END IF;
    RETURN NEW;
END;
} LANGUAGE plpgsql;

CREATE TRIGGER trg_auto_fill_patient BEFORE INSERT ON datasheet
    FOR EACH ROW EXECUTE FUNCTION auto_fill_patient_id();


-- ============================================
-- 6. 视图定义
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

-- ============================================
-- 7. 初始化基础数据
-- ============================================

-- 插入角色 (已剔除放弃的 patient 角色)
INSERT INTO roles (id, name, description, is_system) VALUES 
    ('00000000-0000-0000-0000-000000000001', 'super_admin', '系统管理员（拥有所有模块权限）', true);

INSERT INTO roles (name, description) VALUES 
    ('doctor', '医生'),
    ('nurse', '护士'),
    ('caregiver', '照护者');

-- 插入模块定义
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

-- 赋予超级管理员所有模块权限
INSERT INTO role_modules (role_id, module_id)
SELECT r.id, m.id FROM roles r, modules m WHERE r.name = 'super_admin';

-- 赋予医生、护士核心模块与特征模块权限
INSERT INTO role_modules (role_id, module_id)
SELECT r.id, m.id FROM roles r, modules m 
WHERE r.name IN ('doctor', 'nurse') AND m.category IN ('core', 'feature');

-- 赋予照护者部分核心模块权限
INSERT INTO role_modules (role_id, module_id)
SELECT r.id, m.id FROM roles r, modules m 
WHERE r.name = 'caregiver' AND m.code IN ('dashboard', 'patients', 'bindings', 'data', 'pressure_ulcer');

COMMIT;
