# ccNexus 重构计划文档

## 目录说明

本目录包含 ccNexus 重构计划的相关文档。

## 文档列表

### 1. PRD.md（产品需求文档）

**内容**
- Problem Statement：用户面临的问题
- Solution：解决方案
- User Stories：80 个用户故事，涵盖所有功能模块
- Implementation Decisions：架构设计、数据库设计、端点轮换策略、API 格式转换、统计系统、WebDAV 同步、前端设计
- Testing Decisions：测试策略、测试重点、测试工具
- Out of Scope：明确排除的功能
- Further Notes：性能考虑、安全考虑、可扩展性、用户体验

**用途**
- 定义重构后的功能需求
- 指导开发和测试工作
- 作为验收标准

### 2. FEATURE_IMPLEMENTATION_INDEX.md（功能与实现索引）

**内容**
- 24 个功能模块的详细实现索引
- 每个模块的功能描述、代码实现表格、关键逻辑代码
- 数据库表结构和索引优化
- API 端点和路由注册
- 性能优化和安全考虑
- 测试覆盖和部署配置
- 依赖管理和文档参考
- 功能与实现对照表
- 开发指南和代码规范

**用途**
- 了解旧版功能实现方式
- 识别可复用的代码和设计
- 确定重构重点和优先级
- 避免重复已知的问题

## 旧版参考

**项目路径**
```
E:\myCode\localAway\ccNexus
```

**旧版技术栈**
- 后端：Go 1.24+
- 桌面框架：Wails v2
- 前端：Vue.js
- 数据库：SQLite（modernc.org/sqlite）
- WebDAV 客户端：studio-b12/gowebdav
- 系统托盘：energye/systray

**主要功能**
- 多端点轮换与故障转移
- API 格式转换（Claude ↔ OpenAI Chat）
- 实时统计与监控
- WebDAV 云同步
- 跨平台 GUI
- 系统托盘支持
- 主题系统
- 多语言支持
- 端点筛选和克隆
- 端点测试功能
- 自动更新检查

## 重构目标

### 1. 架构优化

**运行模式**
- 仅支持桌面模式：跨平台 GUI 应用（Windows、macOS、Linux）
- 不支持服务器模式，简化部署和维护
- 具体技术栈待定

**设计原则**
- 模块化设计，便于维护和扩展
- 清晰的接口定义
- 合理的依赖管理
- 良好的代码组织

### 2. 功能精简

**保留的功能**
- 多端点轮换与故障转移
- API 格式转换（Claude ↔ OpenAI Chat）
- 实时统计
- WebDAV 同步
- 系统托盘
- 主题系统
- 模型列表 API
- 健康检查
- Token 计数
- 多语言支持
- 端点筛选和克隆
- 端点测试
- 自动更新

**移除的功能**
- Token Pool 故障处理
- Codex Token Pool
- Token 自动轮换
- Token 状态管理
- Token 刷新机制

### 3. 性能提升

- 优化数据库查询
- 改进缓存策略
- 减少不必要的计算
- 提高响应速度

### 4. 用户体验

- 现代化界面设计
- 流畅的交互体验
- 完善的错误处理
- 友好的提示信息

### 5. 代码质量

- 清晰的代码结构
- 完善的注释文档
- 全面的测试覆盖
- 良好的可维护性

## 重构计划

### 阶段一：架构设计

**目标**
- 设计整体架构
- 定义模块接口
- 规划数据结构
- 确定技术选型

**输出**
- 架构设计文档
- 接口定义文档
- 数据库设计文档

### 阶段二：核心功能开发

**目标**
- 实现核心代理功能
- 实现 API 格式转换（Claude ↔ OpenAI Chat）
- 实现数据存储
- 实现基本配置

**输出**
- 核心功能代码
- 单元测试
- 集成测试

**API 格式转换说明**
- 仅实现 Claude ↔ OpenAI Chat 格式互转
- 转换器接口设计为可扩展，便于后续添加新格式
- 支持流式和非流式响应转换
- 支持工具调用和思考/推理内容转换

### 阶段三：统计与同步

**目标**
- 实现实时统计
- 实现 WebDAV 同步
- 实现数据备份恢复
- 实现多设备同步

**输出**
- 统计功能代码
- 同步功能代码
- 测试用例

### 阶段四：用户界面

**目标**
- 实现桌面 GUI
- 实现系统托盘
- 实现主题系统
- 实现多语言支持

**输出**
- 前端代码
- 界面设计
- 用户测试

### 阶段五：完善与优化

**目标**
- 实现端点管理功能
- 实现自动更新
- 性能优化
- 安全加固

**输出**
- 完整功能代码
- 性能测试报告
- 安全审计报告

## 开发指南

### 开发环境

**后端开发**
```bash
# 安装 Go 1.24+
# 安装桌面框架（待定）
# 克隆项目
# 运行开发服务器
```

**前端开发**
```bash
# 安装 Node.js 18+
# 安装依赖
# 运行开发服务器
```

### 代码规范

**命名规范**
- 使用有意义的变量名
- 遵循语言命名约定
- 保持一致性

**注释规范**
- 解释复杂逻辑
- 说明设计决策
- 提供使用示例

**测试规范**
- 编写单元测试
- 编写集成测试
- 保持测试覆盖率

### 版本控制

**分支管理**
- main：主分支
- develop：开发分支
- feature/*：功能分支
- hotfix/*：热修复分支

**提交规范**
- 使用清晰的提交信息
- 每个提交解决一个问题
- 保持提交原子性

## 参考资源

### 旧版项目文档

- README.md：项目介绍
- CLAUDE.md：开发指南
- docs/configuration.md：配置文档
- docs/development.md：开发文档
- docs/FAQ.md：常见问题

### 技术文档

**Go 语言**
- Go 官方文档：https://golang.org/doc/
- Go 标准库：https://pkg.go.dev/std

**桌面框架（重构时可选）**
- Wails：https://wails.io/（旧版使用）
- Electron：https://www.electronjs.org/
- Tauri：https://tauri.app/

**前端框架（重构时可选）**
- Vue.js：https://vuejs.org/（旧版使用）
- React：https://reactjs.org/
- Svelte：https://svelte.dev/

**SQLite**
- SQLite 官方文档：https://www.sqlite.org/docs.html
- Go SQLite 驱动：https://pkg.go.dev/modernc.org/sqlite

### 工具和库

**开发工具**
- IDE：GoLand、VS Code
- 调试工具：Delve、Chrome DevTools
- 性能分析：pprof、trace

**第三方库**
- WebDAV 客户端：https://github.com/studio-b12/gowebdav
- 系统托盘：https://github.com/energye/systray
- UUID 生成：https://github.com/google/uuid

## 总结

本重构计划基于旧版 ccNexus 项目的经验，旨在构建一个更优质、更易维护的 API 端点轮换代理。通过合理的架构设计、精简的功能集、优化的性能和良好的用户体验，新系统将更好地满足用户需求。

重构过程中，应充分利用旧版项目的参考价值，同时避免重复已知的问题。通过分阶段实施，确保项目按时交付并达到预期质量。
