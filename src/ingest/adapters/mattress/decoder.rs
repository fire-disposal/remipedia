//! 床垫协议解码器

use crate::errors::{AppError, AppResult};
use crate::ingest::protocol::ProtocolDecoder;
use crc::{Crc, CRC_8_SMBUS};

pub struct MattressProtocolDecoder;

impl MattressProtocolDecoder {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MattressProtocolDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolDecoder for MattressProtocolDecoder {
    fn try_decode(&self, buffer: &mut Vec<u8>) -> AppResult<Option<Vec<u8>>> {
        if buffer.len() < 4 {
            return Ok(None);
        }

        // 查找magic头
        if buffer[0] != 0xAB || buffer[1] != 0xCD {
            // 查找下一个可能的magic位置
            for i in 1..buffer.len().saturating_sub(1) {
                if buffer[i] == 0xAB && buffer[i + 1] == 0xCD {
                    buffer.drain(..i);
                    return self.try_decode(buffer);
                }
            }
            buffer.clear();
            return Ok(None);
        }

        let data_len = buffer[2] as usize;
        let total_len = 4 + data_len;

        if buffer.len() < total_len {
            return Ok(None);
        }

        // CRC校验
        let expected_crc = buffer[3];
        let data_bytes = &buffer[4..4 + data_len];

        let crc = Crc::<u8>::new(&CRC_8_SMBUS);
        let computed = crc.checksum(data_bytes);

        if computed != expected_crc {
            log::warn!(
                "CRC校验失败: expected={}, computed={}",
                expected_crc,
                computed
            );
            // 继续处理，不阻止数据流
        }

        let packet = buffer[..total_len].to_vec();
        buffer.drain(..total_len);

        Ok(Some(packet))
    }

    fn extract_serial(&self, decoded: &[u8]) -> AppResult<String> {
        if decoded.len() < 5 {
            return Err(AppError::ValidationError("数据包太短".into()));
        }

        let data = &decoded[4..];

        // 尝试MessagePack解码
        let value: serde_json::Value = rmp_serde::from_slice(data)
            .map_err(|e| AppError::ValidationError(format!("MessagePack解码失败: {}", e)))?;

        value
            .get("sn")
            .or_else(|| value.get("serial_number"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::ValidationError("缺少sn字段".into()))
    }
}
