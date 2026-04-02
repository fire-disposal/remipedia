-- ============================================
-- Ingest 原始数据归档回滚
-- Migration: 20260402093000_ingest_raw_archive
-- ============================================

DROP TRIGGER IF EXISTS update_ingest_raw_updated_at ON ingest_raw_data;
DROP TABLE IF EXISTS ingest_raw_data;
