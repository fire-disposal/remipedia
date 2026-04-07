# 代码改进清单

## 高优先级

- [x] UUID可读性: 0x0000_0000_..._0001 格式 ✅
- [ ] Clippy配置: 创建.clippy.toml减少噪音警告

## 中优先级

- [ ] Self关键字: impl块中使用Self代替显式类型名
- [ ] 文档格式: 类型名添加反引号

## 低优先级

- [ ] let...else语法: 简化模式匹配
- [ ] pub(crate)检查: 私有模块中的可见性
- [ ] map_or_else: 语法糖重构（建议跳过）

## 架构优化

**Repository层**
- [ ] 统一错误处理模式
- [ ] 评估IntoResponse实现

**Service层**
- [ ] 批量查询优化评估
- [ ] DTO转换模式统一

**测试覆盖**
- [ ] Service集成测试
- [ ] Repository数据库测试 (sqlx::test)
- [ ] API端到端测试

## 工具配置

**Clippy配置** (.clippy.toml):
```toml
allow-needless-raw-string-hashes = true
allow-unreadable-literal = true
allow-doc-markdown = true
```

**CI/CD**
- [ ] clippy检查
- [ ] 代码覆盖率
- [ ] cargo audit安全检查

## 性能优化

**数据库**
- [ ] SQL查询索引审查
- [ ] 连接池监控
- [ ] 查询缓存评估

**异步**
- [ ] spawn任务错误处理审查
- [ ] tokio::select!并发优化

## 安全

- [ ] unwrap()/expect()审查（已部分完成）
- [ ] 输入验证测试
- [ ] secrecy crate敏感数据处理
- [ ] 速率限制测试

## 文档

- [ ] API示例补充
- [ ] 架构文档更新
- [ ] 贡献指南

## 核心建议

**保持现状**:
- Repository层实现
- 错误处理模式 (AppError)
- DTO转换分工
- SQL原始字符串格式

**值得投入**:
- 测试覆盖 (当前43个 → 目标80+)
- 集成测试
- 监控指标 (响应时间/错误率)

**代码质量**: 编译通过, 测试通过, 架构清晰

---

**维护者**: Remipedia Team
