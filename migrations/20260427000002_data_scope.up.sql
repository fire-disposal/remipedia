-- data_scope 资源隔离
-- 为 roles 表添加数据范围控制字段

ALTER TABLE roles ADD COLUMN data_scope VARCHAR(16) NOT NULL DEFAULT 'all';

-- 为现有角色设置合理的默认值
UPDATE roles SET data_scope = 'all' WHERE is_system_role = true;
UPDATE roles SET data_scope = 'self' WHERE is_system_role = false;
