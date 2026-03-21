-- ============================================
-- IoT Health Platform 数据库初始化
-- Migration: 20260321000000_init
-- ============================================

-- ============================================
-- 1. 用户表
-- ============================================
CREATE TABLE "user" (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    role            TEXT NOT NULL DEFAULT 'user',
    phone           TEXT UNIQUE,
    email           TEXT UNIQUE,
    avatar_url      TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    last_login_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE "user" IS '系统用户表';
COMMENT ON COLUMN "user".role IS '用户角色：admin(管理员), user(普通用户)';
COMMENT ON COLUMN "user".status IS '用户状态：active(活跃), inactive(未激活), locked(锁定)';

CREATE INDEX idx_user_username ON "user"(username);
CREATE INDEX idx_user_phone ON "user"(phone);
CREATE INDEX idx_user_email ON "user"(email);
CREATE INDEX idx_user_role ON "user"(role);
CREATE INDEX idx_user_status ON "user"(status);

-- ============================================
-- 2. 患者表（极简版）
-- ============================================
CREATE TABLE patient (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    external_id     TEXT UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE patient IS '患者信息表（极简版），仅作为数据归属主体';
COMMENT ON COLUMN patient.external_id IS '外部系统引用 ID，如医院系统 ID';

CREATE INDEX idx_patient_external_id ON patient(external_id);

-- ============================================
-- 3. 患者档案表
-- ============================================
CREATE TABLE patient_profile (
    patient_id      UUID PRIMARY KEY REFERENCES patient(id) ON DELETE CASCADE,
    
    -- 人口统计
    date_of_birth   DATE,
    gender          TEXT,
    blood_type      TEXT,
    
    -- 联系方式
    contact_phone   TEXT,
    address         TEXT,
    emergency_contact TEXT,
    emergency_phone TEXT,
    
    -- 医疗信息
    medical_id      TEXT UNIQUE,
    allergies       JSONB DEFAULT '[]',
    medical_history JSONB DEFAULT '[]',
    
    -- 扩展信息
    notes           TEXT,
    tags            JSONB DEFAULT '[]',
    metadata        JSONB DEFAULT '{}',
    
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE patient_profile IS '患者档案表，存储详细信息';

CREATE INDEX idx_patient_profile_medical_id ON patient_profile(medical_id);
CREATE INDEX idx_patient_profile_tags ON patient_profile USING GIN(tags);

-- ============================================
-- 4. 用户-患者绑定表（预留）
-- ============================================
CREATE TABLE user_patient_binding (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    patient_id      UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    relation        TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT uq_user_patient UNIQUE (user_id, patient_id)
);

COMMENT ON TABLE user_patient_binding IS '用户-患者绑定表，记录用户可访问的患者';
COMMENT ON COLUMN user_patient_binding.relation IS '用户与患者的关系：self(本人), parent(父母), child(子女), caregiver(照护者) 等';

CREATE INDEX idx_user_patient_binding_user ON user_patient_binding(user_id);
CREATE INDEX idx_user_patient_binding_patient ON user_patient_binding(patient_id);

-- ============================================
-- 5. 设备表
-- ============================================
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

COMMENT ON TABLE device IS '设备信息表';
COMMENT ON COLUMN device.serial_number IS '设备序列号，用于自动注册和唯一标识';
COMMENT ON COLUMN device.device_type IS '设备类型，对应 Rust 枚举 DeviceType';
COMMENT ON COLUMN device.status IS '设备状态：active(活跃), inactive(未激活), maintenance(维护中)';

CREATE INDEX idx_device_serial ON device(serial_number);
CREATE INDEX idx_device_status ON device(status);
CREATE INDEX idx_device_type ON device(device_type);

-- ============================================
-- 6. 设备-患者绑定表
-- ============================================
CREATE TABLE binding (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id       UUID NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    patient_id      UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at        TIMESTAMPTZ,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE binding IS '设备-患者绑定关系表，支持时分复用';
COMMENT ON COLUMN binding.ended_at IS '绑定结束时间，NULL 表示当前有效绑定';

CREATE UNIQUE INDEX idx_binding_unique_active ON binding(device_id) WHERE ended_at IS NULL;
CREATE INDEX idx_binding_device ON binding(device_id);
CREATE INDEX idx_binding_patient ON binding(patient_id);
CREATE INDEX idx_binding_time ON binding(started_at, ended_at);

-- ============================================
-- 7. 时间序列数据表
-- ============================================
CREATE TABLE datasheet (
    time            TIMESTAMPTZ NOT NULL,
    device_id       UUID NOT NULL REFERENCES device(id) ON DELETE CASCADE,
    subject_id      UUID REFERENCES patient(id) ON DELETE SET NULL,
    data_type       TEXT NOT NULL,
    payload         JSONB NOT NULL,
    source          TEXT NOT NULL DEFAULT 'mqtt',
    ingested_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    PRIMARY KEY (time, device_id)
);

COMMENT ON TABLE datasheet IS '时间序列数据表，核心数据存储';
COMMENT ON COLUMN datasheet.time IS '设备端时间戳，数据生成时间';
COMMENT ON COLUMN datasheet.subject_id IS '数据归属患者 ID，写入时确定';
COMMENT ON COLUMN datasheet.data_type IS '数据类型，对应 Rust 枚举 DataType';
COMMENT ON COLUMN datasheet.payload IS 'JSONB 格式的负载数据，结构灵活';
COMMENT ON COLUMN datasheet.source IS '数据来源：mqtt, http, tcp 等';
COMMENT ON COLUMN datasheet.ingested_at IS '数据入库时间';

CREATE INDEX idx_datasheet_device_time ON datasheet(device_id, time DESC);
CREATE INDEX idx_datasheet_subject_time ON datasheet(subject_id, time DESC);
CREATE INDEX idx_datasheet_type_time ON datasheet(data_type, time DESC);
CREATE INDEX idx_datasheet_time ON datasheet(time DESC);
CREATE INDEX idx_datasheet_source ON datasheet(source);

-- ============================================
-- 8. 更新时间触发器
-- ============================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_user_updated_at BEFORE UPDATE ON "user"
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_patient_updated_at BEFORE UPDATE ON patient
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_patient_profile_updated_at BEFORE UPDATE ON patient_profile
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_device_updated_at BEFORE UPDATE ON device
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();