# Faasd 前端重写补充文档

> 本文档补充说明 v2.0 版本的开发、部署和最佳实践

## 💻 开发命令

```bash
# 启动开发服务器（带热更新）
pnpm dev

# 构建生产版本
pnpm build

# 预览生产构建
pnpm preview

# 代码检查
pnpm lint

# 代码格式化
pnpm fmt
```

## 🚀 部署指南

### Nginx 配置示例

```nginx
server {
    listen 80;
    server_name yourdomain.com;
    root /path/to/web/dist;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
    }
}
```

### Docker 部署

```dockerfile
FROM node:18-alpine AS builder
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install
COPY . .
RUN pnpm build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE 80
```

## 📝 变更日志

### v2.0.0 (2025-12-03)

**🎯 核心改进**

1. **类型系统重构**
   - 新增 `src/types/index.ts` 集中管理类型
   - 提供 `extractErrorMessage()` 统一错误处理
   - 所有组件使用严格类型定义

2. **Toast 通知系统**
   - 集成 Radix UI Toast
   - 所有用户操作有即时反馈
   - 支持成功/错误/默认三种样式

3. **状态管理改进**
   - 从 `useRef` 迁移到 `useState<UserInfo>`
   - 消除 props drilling 问题
   - 更好的类型安全和响应式更新

4. **新增 UI 组件**
   - `ScrollArea` - 优雅滚动列表
   - `Textarea` - 多行文本输入
   - `Alert` - 警告提示组件
   - `Toaster` - Toast 容器

5. **用户体验提升**
   - Output 组件：自动 JSON 格式化 + 复制功能
   - InvokeForm：使用 Textarea 编辑 JSON
   - 删除操作：防抖保护（500ms）
   - 空状态引导：无函数时显示引导卡片

**🐛 修复**

- 修复 ScrollArea 组件缺失导致的编译错误
- 修复 Props 类型不匹配问题
- 修复表单提交后未刷新列表
- 修复错误处理不统一的问题

**📦 依赖更新**

- 添加 `@radix-ui/react-scroll-area@^1.2.2`
- 所有依赖保持最新稳定版本

## 🎨 代码规范

### TypeScript

```typescript
// ✅ 推荐
interface UserProps {
  userInfo: UserInfo;
  onLogout: () => void;
}

// ❌ 避免
interface UserProps {
  user: any;
  logout: Function;
}
```

### React 组件

```typescript
// ✅ 推荐
export function MyComponent({ title }: { title: string }) {
  const { toast } = useToast();
  
  const handleClick = () => {
    toast({ title: "Success", variant: "success" });
  };
  
  return <Button onClick={handleClick}>{title}</Button>;
}

// ❌ 避免
export default function MyComponent(props) {
  return <button onClick={props.onClick}>{props.title}</button>;
}
```

### 错误处理

```typescript
// ✅ 推荐
try {
  await apiCall();
  toast({ title: "成功", variant: "success" });
} catch (err) {
  const message = extractErrorMessage(err);
  toast({ title: "失败", description: message, variant: "destructive" });
}

// ❌ 避免
try {
  await apiCall();
} catch (err) {
  console.log("error"); // 缺少用户反馈
}
```

## 🔐 安全最佳实践

1. **Token 管理**
   - JWT 存储在 localStorage
   - 请求自动附加 Authorization 头
   - 退出时清除所有认证信息

2. **输入验证**
   - 前端验证必填字段
   - 后端需二次验证

3. **HTTPS**
   - 生产环境强制 HTTPS
   - 配置 CSP 头

## 📚 学习资源

- [React 19 文档](https://react.dev/)
- [Tailwind CSS](https://tailwindcss.com/)
- [Radix UI](https://www.radix-ui.com/)
- [shadcn/ui](https://ui.shadcn.com/)

## 🤝 贡献指南

1. Fork 项目
2. 创建特性分支
3. 遵循代码规范
4. 添加必要的类型定义
5. 提交 Pull Request

---

**项目地址**: [faasd-in-rust](https://github.com/kaleidoscope416/faasd-in-rust)

**维护者**: GitHub Copilot

**最后更新**: 2025-12-03
