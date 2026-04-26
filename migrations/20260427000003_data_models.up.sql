-- ============================================
-- 数据模型重构：DataStream + Observations + AlertEvents
-- Migration: 20260427000003_data_models
-- ============================================

-- ============================================
-- 1. data_streams 表
-- 逻辑数据源抽象，解耦设备与数据的关系
-- ============================================
CREATE TABLE data_streams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    stream_type TEXT NOT NULL CHECK (stream_type IN ('metric', 'event')),
    data_type TEXT NOT NULL,
    device_id UUID REFERENCES device(id) ON DELETE SET NULL,
    patient_id UUID REFERENCES patient(id) ON DELETE SET NULL,
    metadata JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_data_streams_patient ON data_streams(patient_id);
CREATE INDEX idx_data_streams_device ON data_streams(device_id);
CREATE INDEX idx_data_streams_type ON data_streams(stream_type, data_type);

COMMENT ON TABLE data_streams IS '逻辑数据源：设备无关的数据流抽象';
COMMENT ON COLUMN data_streams.stream_type IS '流类型: metric(指标) / event(事件)';
COMMENT ON COLUMN data_streams.data_type IS '数据类型: heart_rate, spo2, imu_sensor, vision_fall 等';

-- ============================================
-- 2. observations 表
-- 数值型指标观测数据
-- ============================================
CREATE TABLE observations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stream_id UUID NOT NULL REFERENCES data_streams(id) ON DELETE CASCADE,
    patient_id UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    value_numeric DECIMAL(10,4),
    value_text TEXT,
    metadata JSONB DEFAULT '{}',
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_observations_stream_time ON observations(stream_id, recorded_at DESC);
CREATE INDEX idx_observations_patient_time ON observations(patient_id, recorded_at DESC);

COMMENT ON TABLE observations IS '观测数据：与 DataStream 关联的数值型指标';

-- ============================================
-- 3. alert_events 表
-- 告警/事件数据
-- ============================================
CREATE TABLE alert_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stream_id UUID NOT NULL REFERENCES data_streams(id) ON DELETE CASCADE,
    patient_id UUID NOT NULL REFERENCES patient(id) ON DELETE CASCADE,
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'alert', 'critical')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'acknowledged', 'resolved')),
    value_numeric DECIMAL(10,4),
    value_text TEXT,
    payload JSONB DEFAULT '{}',
    acknowledged_at TIMESTAMPTZ,
    acknowledged_by UUID REFERENCES "user"(id),
    resolved_at TIMESTAMPTZ,
    resolved_by UUID REFERENCES "user"(id),
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_alert_events_patient_status ON alert_events(patient_id, status, recorded_at DESC);
CREATE INDEX idx_alert_events_severity ON alert_events(severity);

COMMENT ON TABLE alert_events IS '告警事件：含确认/解决工作流的状态跟踪';
