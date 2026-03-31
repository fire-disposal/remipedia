//! 协议解码器 trait
//!
//! 用于处理TCP粘包、协议解析等

use crate::errors::{AppError, AppResult};

/// 协议解码器 trait
pub trait ProtocolDecoder: Send + Sync {
    /// 尝试从缓冲区中提取完整的数据包
    ///
    /// # Arguments
    /// * `buffer` - 累积的数据缓冲区
    ///
    /// # Returns
    /// * `Ok(Some(packet))` - 成功提取完整包
    /// * `Ok(None)` - 数据不足，需要更多数据
    /// * `Err(e)` - 解析错误
    fn try_decode(&self, buffer: &mut Vec<u8>) -> AppResult<Option<Vec<u8>>>;

    /// 从解码后的数据中提取设备序列号
    ///
    /// # Arguments
    /// * `decoded` - 解码后的数据
    ///
    /// # Returns
    /// * `Ok(serial)` - 提取的序列号
    /// * `Err(e)` - 提取失败
    fn extract_serial(&self, decoded: &[u8]) -> AppResult<String>;

    /// 从解码后的数据中提取设备类型（可选）
    fn extract_device_type(&self, _decoded: &[u8]) -> Option<String> {
        None
    }
}

/// 简单JSON解码器（无协议头，直接JSON）
pub struct JsonDecoder;

impl ProtocolDecoder for JsonDecoder {
    fn try_decode(&self, buffer: &mut Vec<u8>) -> AppResult<Option<Vec<u8>>> {
        // 尝试找到完整的JSON对象
        // 简单实现：查找第一个完整的{...}或[...]
        let data = String::from_utf8_lossy(buffer);

        // 查找JSON开始
        if let Some(start) = data.find(['{', '[']) {
            let start_char = data.chars().nth(start).unwrap();
            let end_char = if start_char == '{' { '}' } else { ']' };

            // 查找匹配的结束符
            let mut depth = 1;
            let mut pos = start + 1;

            for (idx, ch) in data[start + 1..].chars().enumerate() {
                if ch == start_char {
                    depth += 1;
                } else if ch == end_char {
                    depth -= 1;
                    if depth == 0 {
                        pos = start + 1 + idx + 1;
                        break;
                    }
                }
            }

            if depth == 0 {
                let packet = buffer[start..pos].to_vec();
                buffer.drain(..pos);
                return Ok(Some(packet));
            }
        }

        // 缓冲区太大但无法解析，丢弃部分数据防止内存溢出
        if buffer.len() > 65536 {
            log::warn!("缓冲区过大(64KB)且无法解析，清空缓冲区");
            buffer.clear();
        }

        Ok(None)
    }

    fn extract_serial(&self, decoded: &[u8]) -> AppResult<String> {
        let value: serde_json::Value = serde_json::from_slice(decoded)
            .map_err(|e| AppError::ValidationError(format!("JSON解析失败: {}", e)))?;

        value
            .get("serial_number")
            .or_else(|| value.get("sn"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::ValidationError("缺少serial_number字段".into()))
    }

    fn extract_device_type(&self, decoded: &[u8]) -> Option<String> {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(decoded) {
            value
                .get("device_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    }
}

/// MessagePack + TLV 解码器（床垫设备协议）
pub struct MessagePackDecoder {
    magic: [u8; 2],
}

impl MessagePackDecoder {
    pub fn new() -> Self {
        Self {
            magic: [0xAB, 0xCD],
        }
    }

    pub fn with_magic(magic: [u8; 2]) -> Self {
        Self { magic }
    }
}

impl Default for MessagePackDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolDecoder for MessagePackDecoder {
    fn try_decode(&self, buffer: &mut Vec<u8>) -> AppResult<Option<Vec<u8>>> {
        if buffer.len() < 4 {
            return Ok(None);
        }

        // 查找magic头
        let magic_pos = buffer.windows(2).position(|w| w == self.magic).unwrap_or(0);

        if magic_pos > 0 {
            // 丢弃magic之前的数据（垃圾数据）
            buffer.drain(..magic_pos);
        }

        if buffer.len() < 4 {
            return Ok(None);
        }

        // 检查magic
        if buffer[0] != self.magic[0] || buffer[1] != self.magic[1] {
            // 丢弃第一个字节，继续查找
            buffer.remove(0);
            return self.try_decode(buffer);
        }

        let data_len = buffer[2] as usize;
        let total_len = 4 + data_len;

        if buffer.len() < total_len {
            return Ok(None);
        }

        // CRC校验（第4字节是CRC）
        let _expected_crc = buffer[3];
        let _data_bytes = &buffer[4..4 + data_len];

        // TODO: CRC校验实现

        let packet = buffer[..total_len].to_vec();
        buffer.drain(..total_len);

        Ok(Some(packet))
    }

    fn extract_serial(&self, decoded: &[u8]) -> AppResult<String> {
        // 跳过4字节头，解析MessagePack
        if decoded.len() < 5 {
            return Err(AppError::ValidationError("数据包太短".into()));
        }

        let data = &decoded[4..];
        let value: serde_json::Value = rmp_serde::from_slice(data)
            .map_err(|e| AppError::ValidationError(format!("MessagePack解析失败: {}", e)))?;

        value
            .get("sn")
            .or_else(|| value.get("serial_number"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::ValidationError("缺少sn字段".into()))
    }

    fn extract_device_type(&self, decoded: &[u8]) -> Option<String> {
        if decoded.len() < 5 {
            return None;
        }

        let data = &decoded[4..];
        if let Ok(value) = rmp_serde::from_slice::<serde_json::Value>(data) {
            value
                .get("device_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    }
}

/// 透传解码器（不解析，直接返回）
pub struct PassthroughDecoder;

impl ProtocolDecoder for PassthroughDecoder {
    fn try_decode(&self, buffer: &mut Vec<u8>) -> AppResult<Option<Vec<u8>>> {
        // 直接取走所有数据
        if buffer.is_empty() {
            return Ok(None);
        }
        let data = buffer.clone();
        buffer.clear();
        Ok(Some(data))
    }

    fn extract_serial(&self, _decoded: &[u8]) -> AppResult<String> {
        // 透传模式无法提取序列号，返回空字符串
        // 实际序列号需要从其他地方获取（如topic、URL等）
        Ok("unknown".to_string())
    }
}
