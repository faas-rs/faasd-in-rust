# faasd-in-rust 前端（web/）开发说明

这份文档是当前前端的“交接手册”：讲清楚架构、关键实现点、扩展方式与迭代计划，方便后续同学直接接着开发。

## 1. 项目概览

`web/` 是一个基于 Vite + React + TypeScript 的单页应用（SPA），提供一个轻量管理面板：

- 登录/注册获取 Token
- 在自己的 `namespace` 下管理函数（列出 / 部署 / 更新 / 删除）
- 调用函数并展示响应

## 2. 技术栈

- React 19 + TypeScript
- Vite 7（本地开发 + 代理）
- Tailwind CSS（`src/index.css` 用 CSS variables 驱动主题）
- Radix UI primitives + 本地 `src/components/ui/*` 组件封装
- axios（请求封装与拦截器）

## 3. 本地运行

在仓库根目录执行：

```bash
cd web
pnpm install
pnpm dev
```

构建/预览：

```bash
pnpm build
pnpm preview
```

格式化：

```bash
pnpm fmt
```

## 4. 环境变量与后端联调

默认配置在 `web/.env`：

```env
VITE_BASE_API=/api
```

同时 `vite.config.ts` 配了开发代理：

- 浏览器请求 `/api/*` 会被转发到 `http://localhost:8080/*`
- 这样本地开发时避免 CORS，也避免把后端地址写死进代码

如果你的后端不在 `localhost:8080`，改 `vite.config.ts` 的 `server.proxy['/api'].target` 即可。

## 5. 架构与数据流

### 5.1 入口与顶层状态

- `src/main.tsx`：React 渲染入口
- `src/App.tsx`：应用顶层状态（`logined / mode / userInfo / loading`）

当前没有引入路由库：登录页与主页面通过 `App` 内的条件渲染切换。

### 5.2 API 层（统一鉴权与响应）

核心在 `src/http.ts`：

- `service`：axios 实例，`baseURL = import.meta.env.VITE_BASE_API`
- 请求拦截器：从 `localStorage.getItem('token')` 读 token，自动加 `Authorization: Bearer <token>`
- GET 防缓存：自动加 `_t=Date.now()`
- 响应拦截器：直接 `return data`（上层拿到的是业务数据，不是 AxiosResponse）

业务接口方法：

- 鉴权：`authLogin`、`authRegister`
- 函数：`getFunctionsList` / `deployFunction` / `updateFunction` / `deleteFunction`
- 调用：`invokeFunction`（POST `/function/${functionName}.${namespace}${route}`）

### 5.3 页面层（“函数管理面板”）

`src/mainpage.tsx` 是核心页面，布局为：

- 顶部 Header：应用标题 + 用户菜单（`src/user.tsx`）
- 左侧 Sidebar：部署/刷新按钮 + 函数列表（`FunctionItem`）
- 主区：空态 / 详情面板（`FunctionInfo`）

`src/function.tsx` 负责函数详情与操作：

- 调用（打开 `InvokeForm`）
- 更新（打开 `Form`，`formType="update"`）
- 删除（`useDebounce` 防止连续点击）

调用响应由 `src/output.tsx` 展示：

- 自动识别 JSON 并 pretty-print
- 一键复制到剪贴板 + toast 提示

### 5.4 UI/交互层

通用 UI 组件在 `src/components/ui/*`，toast 能力在 `src/hooks/use-toast.ts`。

约定：

- 尽量复用 `components/ui`，不要在业务组件里重复写按钮/对话框样式
- 对话框型表单统一放 `src/form.tsx`（目前包含 Deploy/Update 与 Invoke 两套表单）

## 6. 目录结构（最重要的文件）

```
web/
  .env
  vite.config.ts
  src/
    main.tsx            # 入口
    App.tsx             # 顶层状态与登录/注册切换
    http.ts             # API 封装（token、拦截器、接口）
    login.tsx           # 登录
    register.tsx        # 注册
    mainpage.tsx        # 主面板布局 + 列表/详情
    function.tsx        # 函数详情（调用/更新/删除）
    form.tsx            # Deploy/Update/Invoke 的 Dialog 表单
    output.tsx          # 调用结果展示（JSON pretty + copy）
    user.tsx            # 用户菜单/退出登录
    debounce.tsx        # useDebounce
    types/index.ts      # 前端类型 + extractErrorMessage
    components/ui/*     # UI 原子组件
```

## 7. 开发与扩展方式

### 7.1 新增一个后端接口

在 `src/http.ts` 增加一个函数，优先使用 `service`（这样自动带 token）：

```ts
export const getSomething = (params: Record<string, any> = {}) => service.get("/system/something", { params });
```

页面中调用：

```ts
const data = await getSomething({ namespace });
```

### 7.2 新增/调整页面交互

目前应用没有路由；如果要新增多个页面，建议优先做两步：

1) 保持现有结构，在 `App.tsx` 增加新的 mode/状态切换
2) 如果页面开始变多，再引入路由（例如 React Router），把 `App` 简化为路由壳

### 7.3 错误处理

统一使用 `extractErrorMessage`（`src/types/index.ts`），避免到处写 `err.response?.data?.message`。

## 8. 已知问题/技术债（方便后续接手）

- `src/http.ts` 与 `src/types/index.ts` 存在重复的类型定义（`AuthResponse` 等），后续可统一到一个地方。
- `src/user.tsx` 里的“取消”按钮目前是空回调（不影响主流程），需要的话可以补成关闭 Dialog。

## 9. 亮点总结

- 统一 axios 拦截器：自动带 token + GET 防缓存，减少重复代码
- 对话框表单复用：部署/更新/调用统一交互模型
- 删除防抖：`useDebounce` 降低误操作风险
- 输出体验好：JSON 自动格式化 + 一键复制 + toast
- UI 组件化：`components/ui` 保持一致性，迭代成本低

## 10. 迭代计划（建议优先级）

### P0（1-2 天）

- 把重复类型收敛（`http.ts` vs `types/index.ts`）
- 明确后端接口契约（补充每个 API 的字段说明/错误码）

### P1（1 周）

- Token 过期处理：401 自动退出/提示；（若后端支持）刷新 token
- 引入更明确的状态模型（例如把“加载中”从全局遮罩细化到按钮级别）

### P2（2-4 周）

- 引入路由与页面拆分（列表页/详情页/设置页），避免 `Mainpage` 继续膨胀
- 增加基础测试（vitest + React Testing Library），覆盖 `http.ts` 与关键组件
