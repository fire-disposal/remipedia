//! 共享 MQTT 运行器
//!
//! 封装 MQTT 客户端创建、订阅、重连循环，
//! 消除 vision.rs / imu.rs 中重复的 MQTT 连接代码。

use crate::errors::{AppError, AppResult};
use rumqttc::{AsyncClient, MqttOptions, QoS};
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
        .map_err(|e| AppError::ValidationError(format!("订阅失败: {}", e)))?;

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
            Ok(rumqttc::Event::Incoming(rumqttc::Incoming::Publish(publish))) => {
                on_publish(publish);
            }
            Ok(rumqttc::Event::Incoming(rumqttc::Incoming::ConnAck(_))) => {
                log::info!("{} MQTT 连接已建立", module_name);
            }
            Ok(rumqttc::Event::Incoming(rumqttc::Incoming::SubAck(_))) => {
                log::info!("{} 订阅已确认", module_name);
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("{} MQTT 错误: {}", module_name, e);
                return Err(AppError::ValidationError(format!("MQTT 错误: {}", e)));
            }
        }
    }
}
