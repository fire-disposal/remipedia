//! 共享 MQTT 运行器
//!
//! 封装 MQTT 客户端创建、订阅、重连循环，
//! 提供 `MqttMessageHandler` trait 消除模块间的重复 MQTT 事件循环代码。
//!
//! # 架构说明
//!
//! - **`run_with_handler`**：适用于无状态模块（如 Vision），每条消息独立处理。
//! - **IMU 模块**：因需要 per-device `HashMap<String, ImuState>` 状态管理，保留自有事件循环。

use async_trait::async_trait;
use crate::errors::{AppError, AppResult};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

/// MQTT 连接参数
#[derive(Clone)]
pub struct MqttParams {
    pub client_id: String,
    pub broker: String,
    pub port: u16,
    pub topic: String,
    pub qos: QoS,
}

// ---------------------------------------------------------------------------
// MQTT 消息处理器 Trait
// ---------------------------------------------------------------------------

/// MQTT 消息处理器 —— 将单条 Pubilsh 消息的处理逻辑抽象为 trait。
///
/// 适用于无状态/共享状态模块（如 Vision），模块实现此 trait 后，
/// 通过 `run_with_handler` 即可获得完整的 MQTT 事件循环。
#[async_trait]
pub trait MqttMessageHandler: Send + Sync + 'static {
    /// 模块名称（用于日志）
    fn module_name(&self) -> &str;

    /// 处理单条 MQTT Publish 消息
    async fn handle(&self, publish: rumqttc::Publish, pool: &PgPool) -> AppResult<()>;
}

/// 创建 MQTT 客户端并订阅主题。
///
/// 返回 `(AsyncClient, EventLoop)`，调用方通过 EventLoop::poll() 处理消息。
pub async fn connect_and_subscribe(params: &MqttParams) -> AppResult<(AsyncClient, rumqttc::EventLoop)> {
    let mut opts = MqttOptions::new(&params.client_id, &params.broker, params.port);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(false);

    let (client, eventloop) = AsyncClient::new(opts, 100);

    client
        .subscribe(&params.topic, params.qos)
        .await
        .map_err(|e| AppError::validation(format!("订阅失败: {}", e)))?;

    log::info!("MQTT 已订阅: {}", params.topic);
    Ok((client, eventloop))
}

/// 启动后台 MQTT 任务（含自动重连）。
///
/// `run` 闭包每次重连时都会被调用，需完成订阅 + 事件循环。
pub fn spawn_mqtt_task<F, Fut>(module_name: String, params: MqttParams, run: F)
where
    F: Fn(MqttParams) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = AppResult<()>> + Send,
{
    tokio::spawn(async move {
        loop {
            log::info!("{} 连接中... ({}:{} on {})", module_name, params.broker, params.port, params.topic);
            match run(params.clone()).await {
                Ok(()) => {
                    log::info!("{} MQTT 正常退出", module_name);
                    break;
                }
                Err(e) => {
                    log::error!("{} MQTT 错误 (5 秒后重连): {}", module_name, e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });
}

/// 通用 MQTT 事件循环处理函数。
///
/// 处理 ConnAck / SubAck / 错误等通用事件，
/// 仅将 Publish 消息通过 `on_publish` 回调暴露给调用方。
/// 使用 `MqttMessageHandler` trait 的通用事件循环运行器。
///
/// 连接 MQTT Broker → 订阅主题 → 循环 poll 事件，
/// 每条 Publish 消息通过 `handler.handle()` 异步处理。
pub async fn run_with_handler(
    params: MqttParams,
    pool: PgPool,
    handler: Arc<dyn MqttMessageHandler>,
) -> AppResult<()> {
    let (_, mut eventloop) = connect_and_subscribe(&params).await?;
    let module_name = handler.module_name().to_string();

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(publish))) => {
                let handler = handler.clone();
                let pool = pool.clone();
                let name = module_name.clone();
                tokio::spawn(async move {
                    if let Err(e) = handler.handle(publish, &pool).await {
                        log::error!("{} 处理消息失败: {}", name, e);
                    }
                });
            }
            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                log::info!("{} MQTT 连接已建立", module_name);
            }
            Ok(Event::Incoming(Incoming::SubAck(_))) => {
                log::info!("{} 订阅已确认", module_name);
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("{} MQTT 错误: {}", module_name, e);
                return Err(AppError::validation(format!("MQTT 错误: {}", e)));
            }
        }
    }
}

/// 通用 MQTT 事件循环处理函数（闭包回调版本）。
///
/// 处理 ConnAck / SubAck / 错误等通用事件，
/// 仅将 Publish 消息通过 `on_publish` 回调暴露给调用方。
///
/// 注：IMU 模块因维护 per-device `ImuState` 状态管理，
/// 使用此闭包版本而非 `run_with_handler`。
pub async fn run_event_loop<F>(
    eventloop: &mut rumqttc::EventLoop,
    module_name: &str,
    on_publish: F,
) -> AppResult<()>
where
    F: Fn(rumqttc::Publish),
{
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(publish))) => {
                on_publish(publish);
            }
            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                log::info!("{} MQTT 连接已建立", module_name);
            }
            Ok(Event::Incoming(Incoming::SubAck(_))) => {
                log::info!("{} 订阅已确认", module_name);
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("{} MQTT 错误: {}", module_name, e);
                return Err(AppError::validation(format!("MQTT 错误: {}", e)));
            }
        }
    }
}
