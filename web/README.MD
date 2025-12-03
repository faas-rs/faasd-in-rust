# Faasd-in-Rust Web Frontend

基于 React + TypeScript + Vite + Tailwind CSS 的前端项目。

## 技术栈

- **React 19** - UI 框架
- **TypeScript 5.9** - 类型安全
- **Vite 7** - 构建工具
- **Axios** - HTTP 客户端
- **Tailwind CSS 3** - 样式框架

## 环境要求

- Node.js >= 18
- pnpm >= 8（推荐）或 npm/yarn

## 快速开始

### 1. 安装依赖

```bash
pnpm install
# 或
npm install
```

### 2. 配置环境变量

创建 `.env` 文件（如果不存在）：

```bash
# API 基础地址
VITE_BASE_API=http://localhost:8080
```

**注意**：修改环境变量后需要重启开发服务器。

### 3. 启动开发服务器

```bash
pnpm dev
# 或
npm run dev
```

开发服务器将在 `http://localhost:5173` 启动。

### 4. 构建生产版本

```bash
pnpm build
# 或
npm run build
```

构建产物将输出到 `dist/` 目录。

### 5. 预览生产构建

```bash
pnpm preview
# 或
npm run preview
```

## 可用脚本

| 命令 | 说明 |
|------|------|
| `pnpm dev` | 启动开发服务器（热重载） |
| `pnpm build` | 构建生产版本 |
| `pnpm preview` | 预览生产构建 |
| `pnpm lint` | 运行 ESLint 检查 |
| `pnpm fmt` | 格式化代码（Prettier） |

## 项目结构

```
web/
├── src/
│   ├── App.tsx           # 主应用组件
│   ├── main.tsx          # 应用入口
│   ├── http.ts           # API 客户端与接口定义
│   ├── debounce.tsx      # 防抖 Hook
│   ├── login.tsx         # 登录页面
│   ├── register.tsx      # 注册页面
│   ├── mainpage.tsx      # 主页面
│   ├── user.tsx          # 用户组件
│   ├── function.tsx      # 函数列表和详情
│   ├── form.tsx          # 表单组件
│   ├── output.tsx        # 输出组件
│   ├── index.css         # 全局样式
│   └── vite-env.d.ts     # 类型定义
├── public/               # 静态资源
├── index.html            # HTML 模板
├── vite.config.ts        # Vite 配置
├── tsconfig.json         # TypeScript 配置
├── tailwind.config.js    # Tailwind CSS 配置
└── package.json          # 项目依赖

```

## 开发说明

### API 配置

所有 API 请求通过 `src/http.ts` 统一管理：

- **自动添加 Token**：从 `localStorage` 读取并自动添加到请求头
- **请求拦截**：GET 请求自动添加时间戳防缓存
- **响应拦截**：统一处理错误和响应数据
- **类型定义**：所有接口都有明确的 TypeScript 类型

### 类型检查

运行 TypeScript 类型检查（不生成文件）：

```bash
npx tsc --noEmit
```

### 代理配置

开发环境下，Vite 已配置代理转发 `/api` 请求到后端：

```typescript
// vite.config.ts
server: {
  proxy: {
    '/api': {
      target: 'http://localhost:8080',
      changeOrigin: true,
      rewrite: (path) => path.replace(/^\/api/, ''),
    }
  }
}
```

如需修改后端地址，可直接编辑 `.env` 文件中的 `VITE_BASE_API`。

## 生产部署

### 方式一：静态文件服务器（推荐）

1. 构建项目：
   ```bash
   pnpm build
   ```

2. 将 `dist/` 目录部署到任意静态服务器（Nginx、Apache、CDN 等）

3. Nginx 配置示例：
   ```nginx
   server {
     listen 80;
     server_name example.com;
     root /var/www/faasd-web/dist;
     index index.html;

     # SPA 路由回退
     location / {
       try_files $uri $uri/ /index.html;
     }

     # 反向代理 API（避免跨域）
     location /system/ {
       proxy_pass http://localhost:8080/system/;
       proxy_set_header Host $host;
       proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
     }

     location /function/ {
       proxy_pass http://localhost:8080/function/;
       proxy_set_header Host $host;
       proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
     }

     location /auth/ {
       proxy_pass http://localhost:8080/auth/;
       proxy_set_header Host $host;
       proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
     }
   }
   ```

### 方式二：Docker 部署

1. 创建 `Dockerfile`：
   ```dockerfile
   FROM node:18-alpine AS builder
   WORKDIR /app
   COPY package*.json ./
   RUN npm install
   COPY . .
   RUN npm run build

   FROM nginx:alpine
   COPY --from=builder /app/dist /usr/share/nginx/html
   COPY nginx.conf /etc/nginx/conf.d/default.conf
   EXPOSE 80
   CMD ["nginx", "-g", "daemon off;"]
   ```

2. 构建并运行：
   ```bash
   docker build -t faasd-web .
   docker run -d -p 80:80 faasd-web
   ```

## 常见问题

### 1. 启动时提示端口占用

修改 Vite 端口：
```bash
pnpm dev -- --port 3000
```

### 2. API 请求失败（CORS 错误）

- 开发环境：确保 `.env` 中的 `VITE_BASE_API` 正确
- 生产环境：使用 Nginx 反向代理，将前后端放在同一域名下

### 3. 类型错误

运行类型检查：
```bash
npx tsc --noEmit
```

### 4. 热重载不生效

- 清除缓存：`rm -rf node_modules/.vite`
- 重启开发服务器

## 技术亮点

### TypeScript 迁移完成

项目已从 JavaScript 完全迁移到 TypeScript：

- ✅ 所有组件都有明确的 Props 类型
- ✅ API 接口有完整的类型定义
- ✅ 使用泛型保证 Hooks 类型安全
- ✅ 启用 TypeScript strict 模式
- ✅ 事件处理器使用 React 标准事件类型

### 性能优化

- 使用 `useMemo` 缓存计算结果
- 使用 `useCallback` 稳定函数引用
- 防抖处理避免频繁请求
- Vite 构建优化（代码分割、压缩、Tree Shaking）

## 贡献指南

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 许可证

MIT