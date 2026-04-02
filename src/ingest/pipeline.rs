//! 数据接入管道
//!
//! 核心组件：有界队列 + 顺序处理

use crate::core::entity::DataPoint;
use crate::core::entity::RawIngestStatus;
use crate::errors::{AppError, AppResult};
use crate::ingest::{
    adapter::{AdapterRegistry, DeviceType},
    resolver::DeviceResolver,
    state::StateManager,
    DataPacket,
};
use crate::repository::{DataRepository, RawDataRepository};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;

/// 管道配置
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// 队列长度（默认50）
    pub queue_size: usize,
    /// 批量存储大小
    pub batch_size: usize,
    /// 是否自动注册设备
    pub auto_register: bool,
    /// 最大状态数
    pub max_states: usize,
    /// 状态空闲超时（秒）
    pub state_idle_timeout_secs: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            queue_size: 50,
            batch_size: 10,
            auto_register: true,
            max_states: 10000,
            state_idle_timeout_secs: 1800, // 30分钟
        }
    }
}

/// 数据接入管道
pub struct IngestionPipeline {
    tx: mpsc::Sender<DataPacket>,
    config: PipelineConfig,
}

impl IngestionPipeline {
    /// 创建新的管道
    pub fn new(
        pool: &PgPool,
        adapter_registry: Arc<AdapterRegistry>,
        config: PipelineConfig,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel(config.queue_size);

        let pool = pool.clone();
        let max_states = config.max_states;
        let idle_timeout = std::time::Duration::from_secs(config.state_idle_timeout_secs);

        // 启动处理任务
        tokio::spawn(async move {
            let state_manager = Arc::new(StateManager::new(max_states, idle_timeout));
            let resolver = DeviceResolver::new(&pool, config.auto_register);
            let data_repo = DataRepository::new(&pool);
            let raw_repo = RawDataRepository::new(&pool);

            let mut batch_buffer: Vec<DataPoint> = Vec::with_capacity(config.batch_size);

            while let Some(packet) = rx.recv().await {
                let raw_id = match raw_repo.archive_received(&packet).await {
                    Ok(id) => Some(id),
                    Err(e) => {
                        log::error!("原始数据归档失败: {}", e);
                        None
                    }
                };

                match Self::process_packet(packet, &adapter_registry, &state_manager, &resolver)
                    .await
                {
                    Ok(points) => {
                        if points.is_empty() {
                            if let Some(id) = raw_id {
                                if let Err(e) = raw_repo
                                    .mark_status(
                                        id,
                                        RawIngestStatus::Ignored,
                                        Some("解析成功但未产出可入库数据"),
                                    )
                                    .await
                                {
                                    log::error!("更新原始数据状态失败: {}", e);
                                }
                            }
                            continue;
                        }

                        batch_buffer.extend(points);

                        if let Some(id) = raw_id {
                            if let Err(e) = raw_repo
                                .mark_status(id, RawIngestStatus::Ingested, None)
                                .await
                            {
                                log::error!("更新原始数据状态失败: {}", e);
                            }
                        }

                        // 批量存储
                        if batch_buffer.len() >= config.batch_size {
                            if let Err(e) = data_repo.insert_datapoints(&batch_buffer).await {
                                log::error!("批量存储失败: {}", e);
                            }
                            batch_buffer.clear();
                        }
                    }
                    Err(e) => {
                        if let Some(id) = raw_id {
                            let status = if matches!(e, AppError::ValidationError(_)) {
                                RawIngestStatus::FormatError
                            } else {
                                RawIngestStatus::ProcessingError
                            };
                            if let Err(mark_err) =
                                raw_repo.mark_status(id, status, Some(&e.to_string())).await
                            {
                                log::error!("更新原始数据状态失败: {}", mark_err);
                            }
                        }
                        log::error!("处理数据包失败: {}", e);
                    }
                }
            }

            // 存储剩余数据
            if !batch_buffer.is_empty() {
                if let Err(e) = data_repo.insert_datapoints(&batch_buffer).await {
                    log::error!("最终批量存储失败: {}", e);
                }
            }
        });

        Self { tx, config }
    }

    /// 提交数据包到管道
    ///
    /// 如果队列满了，返回错误（不会阻塞）
    pub fn submit(&self, packet: DataPacket) -> AppResult<()> {
        match self.tx.try_send(packet) {
            Ok(_) => {
                log::debug!("数据包已提交到管道");
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                log::warn!(
                    "管道队列已满({}/{}), 丢弃数据包",
                    self.config.queue_size,
                    self.config.queue_size
                );
                Err(AppError::ValidationError("管道队列已满".into()))
            }
            Err(e) => {
                log::error!("提交到管道失败: {}", e);
                Err(AppError::InternalError)
            }
        }
    }

    /// 获取管道状态
    pub fn queue_size(&self) -> usize {
        self.tx.capacity()
    }

    /// 处理单个数据包
    async fn process_packet(
        packet: DataPacket,
        adapters: &Arc<AdapterRegistry>,
        state_manager: &Arc<StateManager>,
        resolver: &DeviceResolver<'_>,
    ) -> AppResult<Vec<DataPoint>> {
        // 1. 解析或确定设备信息
        let (device_id, device_type, serial_number) =
            Self::resolve_device_info(&packet, adapters).await?;

        // 2. 获取适配器
        let adapter = adapters
            .get(&device_type)
            .ok_or_else(|| AppError::ValidationError(format!("未找到适配器: {:?}", device_type)))?;

        // 3. 协议解码（如需要）
        let raw = if let Some(decoder) = adapter.protocol_decoder() {
            let mut buffer = packet.raw.clone();
            match decoder.try_decode(&mut buffer)? {
                Some(decoded) => decoded,
                None => {
                    log::debug!("数据包不完整，等待更多数据");
                    return Ok(vec![]);
                }
            }
        } else {
            packet.raw.clone()
        };

        // 4. 解析数据
        let parsed = adapter.parse(&raw)?;

        // 5. 处理数据
        let datapoints = if adapter.is_stateful() {
            // 有状态处理
            let mut state = state_manager
                .load(&device_id)
                .await
                .or_else(|| adapter.create_state())
                .ok_or_else(|| AppError::InternalError)?;

            let points = adapter.process_with_state(parsed, state.as_mut()).await?;

            state_manager.save(&device_id, state).await;

            points
        } else {
            // 无状态处理
            adapter.process(parsed).await?
        };

        // 6. 填充 device_id 和 patient_id
        let (device_uuid, patient_uuid) = resolver.resolve(&serial_number, device_type).await?;

        let populated_points: Vec<DataPoint> = datapoints
            .into_iter()
            .map(|mut p| {
                p.device_id = Some(device_uuid);
                p.patient_id = patient_uuid;
                p
            })
            .collect();

        Ok(populated_points)
    }

    /// 解析设备信息
    async fn resolve_device_info(
        packet: &DataPacket,
        adapters: &AdapterRegistry,
    ) -> AppResult<(String, DeviceType, String)> {
        // 优先使用metadata中已解析的信息
        if let (Some(serial), Some(device_type_str)) =
            (&packet.metadata.serial_number, &packet.metadata.device_type)
        {
            if let Some(device_type) = DeviceType::from_str(device_type_str) {
                return Ok((serial.clone(), device_type, serial.clone()));
            }
        }

        // 尝试从协议解码器提取
        for device_type in adapters.supported_types() {
            if let Some(adapter) = adapters.get(&device_type) {
                if let Some(decoder) = adapter.protocol_decoder() {
                    let mut buffer = packet.raw.clone();
                    if let Some(decoded) = decoder.try_decode(&mut buffer)? {
                        if let Ok(serial) = decoder.extract_serial(&decoded) {
                            return Ok((serial.clone(), device_type, serial));
                        }
                    }
                }
            }
        }

        // 无法解析
        Err(AppError::ValidationError("无法从数据包解析设备信息".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_queue_full() {
        // 创建小队列测试
        let config = PipelineConfig {
            queue_size: 2,
            ..Default::default()
        };

        // 注意：这里需要实际的pool和registry，实际测试需要集成测试
    }
}
