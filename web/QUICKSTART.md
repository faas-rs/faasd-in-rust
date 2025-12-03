# 🚀 快速启动指南

本文档帮助你快速启动 Faasd 前端项目。

## 📋 前置要求

- Node.js >= 18.0.0
- pnpm >= 8.0.0 (推荐) 或 npm/yarn
- 后端服务已启动（可选，用于完整功能测试）

## 🏃 快速开始

### 1. 安装依赖

```bash
cd web
pnpm install
```

### 2. 配置环境变量

创建 `.env` 文件（如果不存在）：

```bash
# API 基础路径
VITE_BASE_API=/api
```

### 3. 启动开发服务器

```bash
pnpm dev
```

应用将在 http://localhost:5173 打开。

### 4. 构建生产版本

```bash
pnpm build
```

构建产物在 `dist/` 目录。

## 🧪 验证安装

启动后，你应该看到：

1. **登录页面** - 输入用户名和密码
2. **注册功能** - 点击"注册"按钮切换
3. **主控制台** - 登录成功后显示函数管理界面

## ⚙️ 可用命令

| 命令 | 说明 |
|------|------|
| `pnpm dev` | 启动开发服务器（热更新） |
| `pnpm build` | 构建生产版本 |
| `pnpm preview` | 预览生产构建 |
| `pnpm lint` | 检查代码规范 |
| `pnpm fmt` | 格式化代码 |

## 🔧 常见问题

### 端口被占用

修改 `vite.config.ts`：

```typescript
export default defineConfig({
  server: {
    port: 3000, // 改为其他端口
  }
})
```

### API 请求失败

1. 确认后端服务已启动
2. 检查 `.env` 中的 `VITE_BASE_API` 配置
3. 查看浏览器控制台网络请求

### TypeScript 错误

运行类型检查：

```bash
npx tsc --noEmit
```

## 📚 下一步

- 阅读 [FRONTEND_REWRITE_NOTES.md](./FRONTEND_REWRITE_NOTES.md) 了解架构
- 阅读 [DEPLOYMENT.md](./DEPLOYMENT.md) 了解部署方案
- 查看 [src/types/index.ts](./src/types/index.ts) 了解类型定义

## 🤝 需要帮助？

- 查看文档：`FRONTEND_REWRITE_NOTES.md`
- 提交 Issue：GitHub Issues
- 查看日志：浏览器开发者工具控制台

---

**祝开发愉快！** 🎉
