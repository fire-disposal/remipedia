#[cfg(test)]
mod tests {
    use crate::ingest::adapters::smart_mattress::{SmartMattressAdapter, MattressData, TurnOverState};
    use crate::ingest::adapters::DeviceAdapter;
    
    /// 创建测试用的智能床垫适配器
    fn create_adapter() -> SmartMattressAdapter {
        SmartMattressAdapter::new()
    }
    
    /// 创建有效的TCP数据包
    fn create_valid_tcp_packet() -> Vec<u8> {
        // MessagePack数据
        let data = rmp_serde::to_vec(&serde_json::json!({
            "Ma": "HT",
            "Mo": "02",
            "V": 1,
            "Sn": "Z50001",
            "D": {
                "fv": 1,
                "St": "on",
                "Hb": 75,
                "Br": 16,
                "Wt": false,
                "Od": 0,
                "We": 18,
                "P": [6, 9]
            }
        })).unwrap();
        
        let data_len = data.len() as u8;
        
        // 计算CRC
        let crc_algo = crc::Crc::<u8>::new(&crc::CRC_8_SMBUS);
        let crc_value = crc_algo.checksum(&data);
        
        // 构建完整的数据包
        let mut packet = vec![0xab, 0xcd, data_len, crc_value];
        packet.extend_from_slice(&data);
        
        packet
    }
    
    #[test]
    fn test_parse_valid_tcp_packet() {
        let adapter = create_adapter();
        let packet = create_valid_tcp_packet();
        
        let result = adapter.parse_tcp_packet(&packet);
        assert!(result.is_ok());
        
        let data = result.unwrap();
        assert_eq!(data.manufacturer, "HT");
        assert_eq!(data.model, "02");
        assert_eq!(data.serial_number, "Z50001");
        assert_eq!(data.status, "on");
        assert_eq!(data.heart_rate, 75);
        assert_eq!(data.breath_rate, 16);
        assert_eq!(data.wet_status, false);
        assert_eq!(data.weight_value, 18);
        assert_eq!(data.position, [6, 9]);
    }
    
    #[test]
    fn test_parse_invalid_magic() {
        let adapter = create_adapter();
        let mut packet = create_valid_tcp_packet();
        packet[0] = 0x00; // 修改魔数
        
        let result = adapter.parse_tcp_packet(&packet);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("无效的魔数"));
    }
    
    #[test]
    fn test_parse_invalid_crc() {
        let adapter = create_adapter();
        let mut packet = create_valid_tcp_packet();
        packet[3] = 0x00; // 修改CRC
        
        let result = adapter.parse_tcp_packet(&packet);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("CRC校验失败"));
    }
    
    #[test]
    fn test_denoise_data() {
        let adapter = create_adapter();
        
        let mut data = MattressData {
            manufacturer: "HT".to_string(),
            model: "02".to_string(),
            version: 1,
            serial_number: "Z50001".to_string(),
            firmware_version: 1,
            status: "on".to_string(),
            heart_rate: 250, // 异常心率
            breath_rate: 3,  // 异常呼吸率
            wet_status: true,
            apnea_count: 0,
            weight_value: 25, // 异常重量值
            position: [6, 9],
        };
        
        let cleaned = adapter.denoise_data(&data);
        
        // 异常值应该被过滤
        assert_eq!(cleaned.heart_rate, 0);
        assert_eq!(cleaned.breath_rate, 0);
        assert_eq!(cleaned.weight_value, -1);
    }
    
    #[test]
    fn test_turn_over_detection() {
        let mut state = TurnOverState::default();
        
        // 第一次更新，不会检测翻身
        let result1 = state.update_and_detect([6, 9]);
        assert!(result1.is_none());
        
        // 第二次更新，位置变化小，不会检测翻身
        let result2 = state.update_and_detect([7, 10]);
        assert!(result2.is_none());
        
        // 第三次更新，位置变化大，检测翻身
        let result3 = state.update_and_detect([10, 15]);
        assert!(result3.is_some());
        
        let event = result3.unwrap();
        assert_eq!(event.position_before, [7, 10]);
        assert_eq!(event.position_after, [10, 15]);
        assert!(event.change_value > 2.0);
    }
    
    #[test]
    fn test_device_adapter_trait() {
        let adapter = create_adapter();
        let packet = create_valid_tcp_packet();
        
        // 测试parse_payload
        let result = adapter.parse_payload(&packet);
        assert!(result.is_ok());
        
        let payload = result.unwrap();
        
        // 测试validate
        let validation_result = adapter.validate(&payload);
        assert!(validation_result.is_ok());
        
        // 测试data_type和device_type
        assert_eq!(adapter.data_type(), "mattress_status");
        assert_eq!(adapter.device_type(), "smart_mattress");
    }
    
    #[test]
    fn test_validate_invalid_data() {
        let adapter = create_adapter();
        
        // 无效的制造商
        let invalid_payload = serde_json::json!({
            "manufacturer": "INVALID",
            "model": "02",
            "serial_number": "Z50001",
            "status": "on"
        });
        
        let result = adapter.validate(&invalid_payload);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("不支持的制造商"));
        
        // 无效的型号
        let invalid_payload2 = serde_json::json!({
            "manufacturer": "HT",
            "model": "99",
            "serial_number": "Z50001",
            "status": "on"
        });
        
        let result2 = adapter.validate(&invalid_payload2);
        assert!(result2.is_err());
        assert!(result2.unwrap_err().to_string().contains("不支持的型号"));
        
        // 无效的状态
        let invalid_payload3 = serde_json::json!({
            "manufacturer": "HT",
            "model": "02",
            "serial_number": "Z50001",
            "status": "invalid_status"
        });
        
        let result3 = adapter.validate(&invalid_payload3);
        assert!(result3.is_err());
        assert!(result3.unwrap_err().to_string().contains("无效的状态值"));
    }
    
    #[test]
    fn test_extract_packet_from_buffer() {
        use crate::ingest::tcp_server::TcpServer;
        
        let mut buffer = Vec::new();
        
        // 测试空缓冲区
        let result = TcpServer::extract_packet(&mut buffer).unwrap();
        assert!(result.is_none());
        
        // 测试不完整的数据包
        let packet = create_valid_tcp_packet();
        buffer.extend_from_slice(&packet[..3]); // 只包含部分数据
        
        let result = TcpServer::extract_packet(&mut buffer).unwrap();
        assert!(result.is_none());
        assert_eq!(buffer.len(), 3); // 数据应该保留在缓冲区中
        
        // 添加剩余数据
        buffer.extend_from_slice(&packet[3..]);
        
        let result = TcpServer::extract_packet(&mut buffer).unwrap();
        assert!(result.is_some());
        assert_eq!(buffer.len(), 0); // 数据包被提取后，缓冲区应该为空
    }
}