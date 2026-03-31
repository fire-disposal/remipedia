# Remipedia 后续改进清单

## 📋 代码质量优化

### 🔴 高优先级

- [ ] **UUID数字可读性优化**
  - 文件: `src/core/value_object/user_role.rs:12`
  - 问题: `0x00000000000000000000000000000001` 缺少分隔符
  - 建议: 改为 `0x0000_0000_0000_0000_0000_0000_0000_0001`

### 🟡 中优先级

- [ ] **移除原始字符串冗余哈希**
  - 多个Repository文件中的SQL字符串使用了 `r#"..."#`
  - 建议: 改为 `r"..."`（当字符串内不含双引号时）
  - 影响文件:
    - [ ] src/repository/audit_log.rs
    - [ ] src/repository/binding.rs
    - [ ] src/repository/data.rs
    - [ ] src/repository/device.rs
    - [ ] src/repository/patient.rs
    - [ ] ...（其他repository文件）

- [ ] **文档格式规范化**
  - 为文档注释中的类型名称添加反引号
  - 例如: "Service" → `` `Service` ``
  - 参见 clippy lint: `doc_markdown`

### 🟢 低优先级（代码风格）

- [ ] **使用 `Self` 关键字**
  - 在impl块中使用 `Self` 代替显式类型名
  - 参见 clippy lint: `use_self`

- [ ] **使用 `map_or_else` 简化代码**
  - 替换某些 `if let Some(x) = ... { ... } else { ... }` 模式
  - 参见 clippy lint: `option_if_let_else`

- [ ] **使用 `let...else` 语法**
  - 现代Rust语法，使某些模式匹配更简洁
  - 参见 clippy lint: `manual_let_else`

- [ ] **检查 `pub(crate)` 可见性**
  - 某些函数标记为 `pub(crate)` 但所在模块是私有的

## 🏗️ 架构优化

### Repository层
- [ ] 考虑使用 `define_repository!` 宏创建更多Repository
- [ ] 统一所有Repository的错误处理模式
- [ ] 评估是否需要为其他实体实现 `IntoResponse`

### Service层
- [ ] 评估其他Service是否需要类似UserService的批量查询优化
- [ ] 统一DTO转换模式（`From` trait vs `IntoResponse` trait）

### 测试覆盖
- [ ] 为关键Service添加集成测试
- [ ] 为Repository层添加数据库测试（使用 `sqlx::test`）
- [ ] 添加端到端API测试

## 🔧 工具配置

### Clippy配置
创建 `.clippy.toml` 或 `clippy.toml`:
```toml
# 根据项目需求调整lint级别
allow-unreadable-literal = false
allow-needless-raw-string-hashes = false
```

或在 `lib.rs` 中添加:
```rust
#![allow(clippy::unreadable_literal)]
#![allow(clippy::needless_raw_string_hashes)]
```

### CI/CD
- [ ] 在CI中添加 clippy 检查
- [ ] 添加代码覆盖率报告
- [ ] 添加安全检查（`cargo audit`）

## 📊 性能优化

### 数据库
- [ ] 审查所有SQL查询，确保有适当的索引
- [ ] 考虑使用连接池监控
- [ ] 评估是否需要查询缓存层

### 异步
- [ ] 审查 `spawn` 任务的错误处理
- [ ] 考虑使用 `tokio::select!` 优化并发

## 🔒 安全

- [ ] 审查所有 `unwrap()` 和 `expect()`（已部分完成）
- [ ] 添加输入验证测试
- [ ] 考虑使用 `secrecy` crate 处理敏感数据
- [ ] 添加速率限制测试

## 📝 文档

- [ ] 为公共API添加更多示例
- [ ] 更新架构文档（如 ARCHITECTURE.md）
- [ ] 添加贡献指南

## 🎯 完成标准

每项改进应满足:
1. 代码通过 `cargo clippy -- -W clippy::all` 检查
2. 所有测试通过 `cargo test`
3. 文档更新（如适用）

---

**最后更新**: 2024年
**维护者**: Remipedia Team
