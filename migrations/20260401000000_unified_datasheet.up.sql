-- ============================================
-- 统一数据表改造
-- Migration: 20260401000000_unified_datasheet
-- ============================================

-- ============================================
-- 1. 更新 datasheet 表，支持事件存储
-- ============================================
ALTER TABLE datasheet 
    ADD COLUMN IF NOT EXISTS patient_id UUID REFERENCES patient(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS data_category TEXT NOT NULL DEFAULT 'metric',
    ADD COLUMN IF NOT EXISTS value_numeric DECIMAL(10,4),
    ADD COLUMN IF NOT EXISTS value_text TEXT,
    ADD COLUMN IF NOT EXISTS severity TEXT,
    ADD COLUMN IF NOT EXISTS status TEXT DEFAULT NULL;

-- 添加约束检查
ALTER TABLE datasheet 
    ADD CONSTRAINT chk_data_category CHECK (data_category IN ('metric', 'event')),
    ADD CONSTRAINT chk_severity CHECK (severity IS NULL OR severity IN ('info', 'warning', 'alert')),
    ADD CONSTRAINT chk_status CHECK (status IS NULL OR status IN ('active', 'acknowledged', 'resolved'));

COMMENT ON COLUMN datasheet.patient_id IS '患者ID，从设备绑定自动填充';
COMMENT ON COLUMN datasheet.data_category IS '数据分类: metric(指标), event(事件)';
COMMENT ON COLUMN datasheet.value_numeric IS '数值型指标（如心率120）';
COMMENT ON COLUMN datasheet.value_text IS '文本型指标（如状态描述）';
COMMENT ON COLUMN datasheet.severity IS '事件严重级别: info/warning/alert';
COMMENT ON COLUMN datasheet.status IS '事件状态: active/acknowledged/resolved';

-- ============================================
-- 2. 创建优化索引
-- ============================================

-- 患者+时间索引（最常用）
CREATE INDEX IF NOT EXISTS idx_datasheet_patient_time 
    ON datasheet(patient_id, time DESC) 
    WHERE patient_id IS NOT NULL;

-- 事件查询索引
CREATE INDEX IF NOT EXISTS idx_datasheet_events 
    ON datasheet(patient_id, time DESC) 
    WHERE data_category = 'event';

-- 严重级别索引
CREATE INDEX IF NOT EXISTS idx_datasheet_severity 
    ON datasheet(severity) 
    WHERE severity IS NOT NULL;

-- 状态索引（用于查询待处理告警）
CREATE INDEX IF NOT EXISTS idx_datasheet_status 
    ON datasheet(status) 
    WHERE status IS NOT NULL;

-- 综合事件查询索引
CREATE INDEX IF NOT EXISTS idx_datasheet_active_alerts 
    ON datasheet(patient_id, time DESC) 
    WHERE data_category = 'event' AND status = 'active';

-- ============================================
-- 3. 迁移现有数据
-- ============================================

-- 为现有数据填充 patient_id（从 binding 表关联）
UPDATE datasheet d
SET patient_id = b.patient_id
FROM binding b
WHERE d.device_id = b.device_id 
  AND b.ended_at IS NULL
  AND d.patient_id IS NULL;

-- ============================================
-- 4. 创建触发器函数：自动填充 patient_id
-- ============================================
CREATE OR REPLACE FUNCTION auto_fill_patient_id()
RETURNS TRIGGER AS $$
BEGIN
    -- 如果 patient_id 为空，尝试从绑定表获取
    IF NEW.patient_id IS NULL AND NEW.device_id IS NOT NULL THEN
        SELECT patient_id INTO NEW.patient_id
        FROM binding
        WHERE device_id = NEW.device_id 
          AND ended_at IS NULL
        ORDER BY started_at DESC
        LIMIT 1;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 创建触发器
DROP TRIGGER IF EXISTS trg_auto_fill_patient ON datasheet;
CREATE TRIGGER trg_auto_fill_patient
    BEFORE INSERT ON datasheet
    FOR EACH ROW
    EXECUTE FUNCTION auto_fill_patient_id();

-- ============================================
-- 5. 插入示例告警数据（用于测试）
-- ============================================

-- 注意：这会在有患者数据时插入测试告警
DO $$
DECLARE
    test_patient_id UUID;
    test_device_id UUID;
BEGIN
    -- 获取第一个患者和设备
    SELECT id INTO test_patient_id FROM patient LIMIT 1;
    SELECT id INTO test_device_id FROM device LIMIT 1;
    
    IF test_patient_id IS NOT NULL AND test_device_id IS NOT NULL THEN
        -- 插入测试告警
        INSERT INTO datasheet (
            time, device_id, patient_id, data_type, data_category,
            value_numeric, value_text, severity, status, payload, source
        ) VALUES 
        (
            NOW() - INTERVAL '1 hour',
            test_device_id,
            test_patient_id,
            'heart_rate_high',
            'event',
            145.00,
            '心率过高',
            'warning',
            'active',
            '{"heart_rate": 145, "threshold": 120, "duration_sec": 30}'::jsonb,
            'system'
        ),
        (
            NOW() - INTERVAL '30 minutes',
            test_device_id,
            test_patient_id,
            'spo2_low',
            'event',
            88.00,
            '血氧偏低',
            'alert',
            'acknowledged',
            '{"spo2": 88, "threshold": 95}'::jsonb,
            'system'
        );
        
        RAISE NOTICE '已插入测试告警数据';
    END IF;
END $$;
