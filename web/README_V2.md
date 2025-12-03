# 📦 Faasd 前端项目 v2.0

> 现代化的 FaaS (Function as a Service) 管理控制台

## ✨ 项目亮点

- 🎯 **完整类型安全** - 100% TypeScript 覆盖
- 🔔 **即时反馈** - Toast 通知覆盖所有用户操作
- 🎨 **现代 UI** - 基于 Radix UI + Tailwind CSS
- ⚡ **极速体验** - Vite 7 构建，HMR 秒级响应
- 📱 **响应式设计** - 完美支持桌面和移动端
- 🛡️ **错误处理** - 统一的错误提取和展示机制

## 🚀 快速开始

```bash
# 1. 安装依赖
cd web && pnpm install

# 2. 启动开发服务器
pnpm dev

# 3. 打开浏览器
# http://localhost:5173
```

详见 [QUICKSTART.md](./QUICKSTART.md)

## 📚 文档导航

| 文档 | 说明 |
|------|------|
| [QUICKSTART.md](./QUICKSTART.md) | 快速启动指南 |
| [FRONTEND_REWRITE_NOTES.md](./FRONTEND_REWRITE_NOTES.md) | 详细架构文档 |
| [DEPLOYMENT.md](./DEPLOYMENT.md) | 部署和最佳实践 |
| [CHANGELOG_V2.md](./CHANGELOG_V2.md) | v2.0 变更总结 |

## 🎯 核心功能

### 认证系统
- ✅ 用户登录/注册
- ✅ JWT Token 管理
- ✅ 自动请求拦截

### 函数管理
- ✅ 函数列表查看
- ✅ 部署新函数
- ✅ 更新函数配置
- ✅ 删除函数（防抖保护）
- ✅ 函数调用测试

### 用户体验
- ✅ Toast 通知反馈
- ✅ JSON 自动格式化
- ✅ 一键复制结果
- ✅ 空状态引导
- ✅ 加载状态指示

## 🛠 技术栈

```json
{
  "框架": "React 19 + TypeScript 5.9",
  "构建": "Vite 7",
  "样式": "Tailwind CSS 3.4",
  "组件": "Radix UI",
  "请求": "Axios 1.12",
  "包管理": "pnpm 10"
}
```

## 📁 项目结构

```
web/
├── src/
│   ├── components/ui/     # UI 组件库
│   ├── hooks/             # 自定义 Hooks
│   ├── types/             # 类型定义
│   ├── App.tsx            # 根组件
│   ├── main.tsx           # 入口文件
│   ├── login.tsx          # 登录页
│   ├── register.tsx       # 注册页
│   ├── mainpage.tsx       # 主控制台
│   ├── function.tsx       # 函数组件
│   ├── form.tsx           # 表单组件
│   ├── output.tsx         # 结果展示
│   ├── user.tsx           # 用户菜单
│   └── http.ts            # API 封装
├── docs/                  # 文档目录
├── .env                   # 环境变量
└── package.json           # 依赖配置
```

## 🎨 UI 组件

### 基础组件
- Button - 按钮（6种变体）
- Input - 输入框
- Textarea - 多行文本
- Card - 卡片容器
- Badge - 徽章标签
- Alert - 警告提示

### 反馈组件
- Toast - 通知消息
- Dialog - 对话框
- Separator - 分隔符

### 布局组件
- ScrollArea - 滚动区域
- Label - 表单标签

## 🔐 安全特性

- JWT Token 认证
- 请求自动拦截
- 输入验证
- HTTPS 强制（生产环境）
- XSS 防护（React 内置）

## 📈 性能优化

- ⚡ Vite 极速构建
- 📦 代码分割和懒加载
- 🎯 Tree-shaking 移除死代码
- 💾 组件级缓存（useMemo）
- 🔄 防抖优化（useDebounce）

## 🧪 开发命令

```bash
pnpm dev      # 开发服务器
pnpm build    # 生产构建
pnpm preview  # 预览构建
pnpm lint     # 代码检查
pnpm fmt      # 代码格式化
```

## 🚢 部署选项

- **Nginx** - 静态托管 + API 代理
- **Docker** - 容器化部署
- **Vercel/Netlify** - 无服务器部署
- **CDN** - 静态资源加速

详见 [DEPLOYMENT.md](./DEPLOYMENT.md)

## 📊 v2.0 变更亮点

### 🎯 新增功能
- ✨ Toast 通知系统（覆盖所有操作）
- 📋 JSON 自动格式化和复制
- 📝 多行 Textarea 编辑 JSON
- 🎨 ScrollArea 优雅滚动
- 🛡️ 统一错误处理机制

### 🔧 架构改进
- 💎 完整的 TypeScript 类型系统
- 🗂️ 类型定义集中管理（types/）
- 🔄 从 useRef 迁移到 useState
- 📦 新增 10 个组件和工具
- 📚 完善的文档体系

### 🐛 问题修复
- 修复 ScrollArea 组件缺失
- 修复 Props 类型不匹配
- 修复表单提交后未刷新
- 修复错误处理不统一

详见 [CHANGELOG_V2.md](./CHANGELOG_V2.md)

## 🤝 贡献指南

1. Fork 本项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交改动 (`git commit -m 'Add some feature'`)
4. 推送分支 (`git push origin feature/AmazingFeature`)
5. 提交 Pull Request

## 📄 许可证

本项目采用 MIT License。

## 👥 维护者

- GitHub Copilot

## 🙏 致谢

- [React](https://react.dev/)
- [Vite](https://vitejs.dev/)
- [Radix UI](https://www.radix-ui.com/)
- [Tailwind CSS](https://tailwindcss.com/)
- [shadcn/ui](https://ui.shadcn.com/)

## 📞 支持

- 📖 文档：查阅项目文档
- 🐛 Bug：提交 GitHub Issue
- 💡 建议：参与 Discussions
- 📧 联系：通过 GitHub

---

**版本**: 2.0.0  
**更新**: 2025-12-03  
**状态**: ✅ 稳定版本

🎉 **感谢使用 Faasd 前端项目！**
