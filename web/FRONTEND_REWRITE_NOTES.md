# Faasd 前端重写文档 v2.0

> **最后更新**: 2025-12-03  
> **版本**: 2.0.0  
> **作者**: GitHub Copilot

本文档详细说明了 Faasd 前端项目的完整重构方案，包括架构设计、技术选型、组件体系和最佳实践。

## 📋 目录

- [技术栈](#技术栈)
- [项目结构](#项目结构)
- [核心改进](#核心改进)
- [组件文档](#组件文档)
- [状态管理](#状态管理)
- [开发指南](#开发指南)
- [部署说明](#部署说明)

---

## 🛠 技术栈

### 核心框架
- **构建工具**: Vite 7.1.7 - 下一代前端构建工具，极速的 HMR
- **UI 框架**: React 19.1.1 - 最新版本，支持 Compiler 和并发特性
- **类型系统**: TypeScript 5.9.3 - 完整的类型安全
- **样式方案**: Tailwind CSS 3.4.18 - 实用优先的 CSS 框架

### UI 组件库
- **基础组件**: Radix UI - 无障碍访问优先的底层组件
  - `@radix-ui/react-dialog` - 对话框
  - `@radix-ui/react-toast` - 通知提示
  - `@radix-ui/react-scroll-area` - 滚动区域
  - `@radix-ui/react-separator` - 分隔符
  - `@radix-ui/react-label` - 表单标签
  - `@radix-ui/react-slot` - 组合式组件
- **图标库**: Lucide React 0.555.0 - 精美的 SVG 图标
- **工具库**: 
  - `class-variance-authority` - 组件变体管理
  - `clsx` & `tailwind-merge` - 类名合并工具

### 网络与状态
- **HTTP 客户端**: Axios 1.12.2 - 强大的请求拦截和响应处理
- **状态管理**: React Hooks - 本地状态 + Context（未来可扩展为 Zustand）

### 开发工具
- **代码规范**: ESLint 9.36.0 + Prettier 3.6.2
- **构建优化**: PostCSS + Autoprefixer

---

## 📁 项目结构

```
web/
├── public/                    # 静态资源
├── src/
│   ├── components/           # UI 组件库
│   │   └── ui/              # shadcn/ui 风格的基础组件
│   │       ├── alert.tsx    # 警告提示组件
│   │       ├── badge.tsx    # 徽章组件
│   │       ├── button.tsx   # 按钮组件
│   │       ├── card.tsx     # 卡片组件
│   │       ├── dialog.tsx   # 对话框组件
│   │       ├── input.tsx    # 输入框组件
│   │       ├── label.tsx    # 标签组件
│   │       ├── scroll-area.tsx  # 滚动区域
│   │       ├── separator.tsx    # 分隔符
│   │       ├── textarea.tsx     # 文本域组件
│   │       ├── toast.tsx        # Toast 通知组件
│   │       └── toaster.tsx      # Toast 容器
│   ├── hooks/               # 自定义 Hooks
│   │   └── use-toast.ts     # Toast 通知 Hook
│   ├── lib/                 # 工具函数
│   │   └── utils.ts         # cn() 类名合并等
│   ├── types/               # 类型定义 ⭐ 新增
│   │   └── index.ts         # 全局类型、接口和工具函数
│   ├── App.tsx              # 根组件 - 路由控制
│   ├── main.tsx             # 应用入口
│   ├── login.tsx            # 登录页面
│   ├── register.tsx         # 注册页面
│   ├── mainpage.tsx         # 主控制台页面
│   ├── function.tsx         # 函数列表与详情组件
│   ├── form.tsx             # 表单组件（部署/更新/调用）
│   ├── output.tsx           # 函数调用结果展示
│   ├── user.tsx             # 用户信息与退出组件
│   ├── http.ts              # HTTP 请求封装
│   ├── debounce.tsx         # 防抖 Hook
│   └── index.css            # 全局样式
├── .env                     # 环境变量
├── package.json             # 依赖管理
├── tsconfig.json            # TypeScript 配置
├── vite.config.ts           # Vite 配置
├── tailwind.config.js       # Tailwind 配置
└── FRONTEND_REWRITE_NOTES.md  # 本文档
```

---

## 🚀 核心改进

本次重写在保持原有功能的基础上，进行了全面的架构升级和用户体验优化。

### 1. 类型系统重构 ✨

**新增 `src/types/index.ts`**

- **统一类型定义**: 所有 API 类型、组件 Props 类型集中管理
- **类型安全增强**: 消除 `any` 类型，全面使用 TypeScript 严格模式
- **错误处理工具**: 提供 `extractErrorMessage()` 统一错误信息提取

```typescript
// 核心类型
export interface UserInfo {
  username: string;
  namespace?: string;
}

export interface FunctionItem {
  functionName: string;
  namespace: string;
  image: string;
}

// 错误处理
export function extractErrorMessage(error: unknown): string {
  // 智能提取错误信息
}
```

### 2. 全局通知系统 🔔

**Toast 通知集成**

- 基于 Radix UI Toast 构建完整通知系统
- 支持成功/错误/警告三种变体
- 自动管理通知队列和过期时间
- 所有用户操作均有即时反馈

**使用示例:**
```typescript
toast({
  title: "操作成功",
  description: "函数已成功部署",
  variant: "success",
});
```

### 3. 组件库完善 🧩

**新增组件:**
- `ScrollArea` - 优雅的滚动区域，用于函数列表
- `Toast` / `Toaster` - 完整的通知系统
- `Textarea` - 多行文本输入，用于 JSON 数据
- `Alert` - 警告提示组件，支持多种样式

**组件增强:**
- `Output` 组件：
  - 自动 JSON 格式化和语法高亮
  - 一键复制功能
  - 最大高度限制 + 滚动条
- `Button` / `Card` / `Input` 等基础组件统一样式规范

### 4. 状态管理优化 📊

**从 `useRef` 迁移到状态管理:**

```typescript
// 旧方案 ❌
const usernameRef = useRef<string>("defaultUser");

// 新方案 ✅
const [userInfo, setUserInfo] = useState<UserInfo>({ 
  username: "",
  namespace: ""
});
```

**好处:**
- 类型安全，编译期检查
- 支持响应式更新
- 便于调试和追踪
- 更符合 React 最佳实践

### 5. 错误处理增强 🛡️

**统一错误处理流程:**

1. **捕获**: 所有异步操作使用 try-catch
2. **提取**: 通过 `extractErrorMessage()` 智能提取错误信息
3. **展示**: Toast 通知 + 表单内联错误
4. **日志**: console.error 保留调试信息

```typescript
try {
  await deployFunction(payload);
  toast({ title: "部署成功", variant: "success" });
} catch (err) {
  const message = extractErrorMessage(err);
  toast({ title: "部署失败", description: message, variant: "destructive" });
}
```

### 6. 用户体验提升 ✨

**表单改进:**
- 更清晰的标签和占位符
- 实时验证和错误提示
- 提交中的加载状态
- Textarea 替代 Input 用于 JSON 输入

**交互优化:**
- 删除操作使用 `useDebounce` 防抖（500ms）
- 所有操作有即时反馈（Toast）
- 响应式设计，支持移动端
- 空状态引导：无函数时显示引导卡片

**视觉改进:**
- 统一配色方案和间距
- 图标与文本对齐
- hover 和 active 状态优化
- 暗色模式友好（Tailwind CSS 支持）

---

## 📄 组件文档

### 🔐 认证系统

#### `App.tsx` - 根组件

**职责:**
- 管理全局认证状态（已登录/未登录）
- 控制登录/注册页面切换
- 集成 Toaster 通知系统
- 全局 Loading 遮罩

**核心状态:**
```typescript
const [logined, setLogined] = useState<boolean>(false);
const [mode, setMode] = useState<Mode>("login" | "register");
const [userInfo, setUserInfo] = useState<UserInfo>({ username: "" });
```

**渲染逻辑:**
- 未登录: 显示 Login 或 Register 组件
- 已登录: 显示 Mainpage 主控制台

#### `login.tsx` - 登录页面

**特性:**
- 表单验证：用户名和密码非空检查
- 错误处理：内联错误 + Toast 通知
- 成功登录：保存 token 到 localStorage，更新 userInfo

**Props:**
```typescript
interface LoginProps {
  loading: boolean;
  setLoading: (value: boolean) => void;
  setLogined: (value: boolean) => void;
  setUserInfo: (userInfo: UserInfo) => void;  // ⭐ 新增
}
```

**用户体验:**
- 登录中禁用表单输入
- 成功时显示欢迎 Toast
- 失败时显示具体错误原因

#### `register.tsx` - 注册页面

**特性:**
- 双重反馈：内联成功/错误消息 + Toast 通知
- 自动跳转：注册成功 1.5 秒后返回登录页
- 表单验证同 Login

**改进点:**
- 移除了不必要的 `setLogined` prop
- 统一使用 `extractErrorMessage()` 处理错误

---

### 🎛️ 主控制台

#### `mainpage.tsx` - 函数管理主页

**布局结构:**
```
┌─────────────────────────────────────────┐
│  Header (系统标题 + 用户信息)            │
├──────────┬──────────────────────────────┤
│  Sidebar │  Main Content                │
│          │                               │
│  • 部署  │  • 空状态提示                 │
│  • 刷新  │  • 选中函数详情               │
│  • 列表  │  • 操作按钮                   │
│          │  • 调用结果                   │
└──────────┴──────────────────────────────┘
```

**核心功能:**
1. **函数列表管理**
   - `fetchList()`: 获取当前用户命名空间下的函数
   - 使用 `useMemo` 计算选中函数，避免重复查找
   - ScrollArea 支持大量函数时的流畅滚动

2. **部署与刷新**
   - `openDeploy()`: 打开部署表单，自动填充 namespace
   - 错误时通过 Toast 反馈

3. **函数选择**
   - 点击函数项切换选中状态
   - 高亮显示当前选中项
   - 双击同一函数取消选中

**Props 更新:**
```typescript
interface MainpageProps {
  userInfo: UserInfo;     // ⭐ 从 useRef 改为 state
  setLogined: (value: boolean) => void;
}
```

#### `function.tsx` - 函数组件

**包含两个子组件:**

1. **`FunctionItem`** - 列表项
   - 显示函数名和图标
   - 支持选中态样式
   - 点击切换选中

2. **`FunctionInfo`** - 详情卡片
   - 显示函数名、namespace、镜像
   - 三大操作：调用、更新、删除
   - 集成 Toast 通知

**删除功能增强:**
```typescript
const handleDelete = useDebounce(async (functionName, namespace) => {
  try {
    await deleteFunction({ functionName, namespace });
    setFunctions(prev => prev.filter(...));
    toast({ title: "删除成功", variant: "success" });
  } catch (err) {
    toast({ title: "删除失败", description: extractErrorMessage(err), variant: "destructive" });
  }
}, 500);
```

---

### 📝 表单系统

#### `form.tsx` - 表单组件集合

**包含两个表单组件:**

1. **`Form`** - 部署/更新表单
   - **formType: "deploy" | "update"**
   - 动态标题和描述
   - 字段：函数名、镜像地址
   - namespace 自动填充

2. **`InvokeForm`** - 调用参数表单
   - 路由配置
   - Content-Type 选择
   - JSON 数据输入（Textarea）⭐ 改进
   - 支持多行 JSON 编辑

**表单提交流程:**
```
用户填写 → 前端验证 → 显示 Loading → 
调用 API → 成功/失败处理 → Toast 通知 → 
关闭表单 → 刷新列表
```

#### `output.tsx` - 调用结果展示

**新特性:**
- ✨ **自动 JSON 格式化**: 检测并美化 JSON 输出
- 📋 **一键复制**: 复制响应内容到剪贴板
- 📊 **视觉优化**: 固定最大高度 + 滚动条
- 🎨 **语法高亮**: 为 JSON 添加 `language-json` 类

```typescript
// 智能 JSON 格式化
try {
  const parsed = JSON.parse(response);
  formattedResponse = JSON.stringify(parsed, null, 2);
  isJson = true;
} catch {
  // 不是 JSON，显示原始文本
}
```

---

### 🧰 工具组件与 Hooks

#### `user.tsx` - 用户菜单

**功能:**
- 显示当前用户名
- 退出确认对话框
- 清除 localStorage 中的 token

**Props 更新:**
```typescript
interface UserProps {
  userInfo: UserInfo;  // ⭐ 替代 username: React.MutableRefObject<string>
  setlogined: (value: boolean) => void;
  className?: string;
}
```

#### `debounce.tsx` - 防抖 Hook

**用途:**
- 防止用户快速连点导致重复请求
- 主要用于删除操作（500ms 延迟）

**类型安全:**
```typescript
export function useDebounce<T extends (...args: any[]) => any>(
  fn: T,
  delay: number
): (...args: Parameters<T>) => void
```

#### `use-toast.ts` - Toast 通知 Hook

**功能:**
- 管理全局通知队列
- 支持最多 1 个同时显示的通知
- 自动移除过期通知
- 提供 `toast()` 和 `dismiss()` 方法

**使用:**
```typescript
const { toast } = useToast();

toast({
  title: "标题",
  description: "描述",
  variant: "success" | "destructive" | "default",
});
```

---

## 🔌 HTTP 请求层

### `http.ts` - API 封装

**Axios 实例配置:**
- **baseURL**: 从环境变量 `VITE_BASE_API` 读取（默认 `/api`）
- **timeout**: 10 秒
- **请求拦截**: 自动附加 JWT token 到 Authorization 头
- **响应拦截**: 统一返回 `response.data`，触发全局事件

**API 列表:**

| 函数名 | 方法 | 路径 | 说明 |
|--------|------|------|------|
| `authLogin` | POST | `/auth/login` | 用户登录 |
| `authRegister` | POST | `/auth/register` | 用户注册 |
| `getFunctionsList` | GET | `/system/functions` | 获取函数列表 |
| `deployFunction` | POST | `/system/functions` | 部署新函数 |
| `updateFunction` | PUT | `/system/functions` | 更新函数配置 |
| `deleteFunction` | DELETE | `/system/functions` | 删除函数 |
| `invokeFunction` | POST | `/function/{name}.{ns}{route}` | 调用函数 |

**请求拦截器:**
```typescript
service.interceptors.request.use((config) => {
  // 1. 附加 JWT token
  const token = localStorage.getItem("token");
  if (token) config.headers.Authorization = `Bearer ${token}`;
  
  // 2. 防止 GET 请求缓存
  if (config.method === "get") {
    config.params = { ...config.params, _t: Date.now() };
  }
  
  return config;
});
```

---

## 📊 状态管理

### 当前方案：React Hooks + Props Drilling

**状态层级:**
```
App (全局状态)
├── userInfo: UserInfo          // 用户信息
├── logined: boolean            // 登录状态
├── loading: boolean            // 全局加载
└── mode: "login"|"register"    // 认证模式

Mainpage (页面状态)
├── functions: FunctionItem[]   // 函数列表
├── selFuncId: string|null      // 选中的函数
├── showDeployForm: boolean     // 表单显示状态
└── form: DeployFormData        // 表单数据

FunctionInfo (组件状态)
├── showUpdateForm: boolean     // 更新表单
├── showInvokeForm: boolean     // 调用表单
├── invokeResponse: string      // 调用结果
└── invokeForm: InvokeFormData  // 调用参数
```

### 未来扩展建议

**如项目规模扩大，可考虑:**

1. **Zustand** - 轻量级状态管理
   ```typescript
   import create from 'zustand';
   
   const useStore = create((set) => ({
     userInfo: null,
     setUserInfo: (userInfo) => set({ userInfo }),
   }));
   ```

2. **React Query** - 服务端状态管理
   - 自动缓存和重新验证
   - 请求去重和后台更新
   - 乐观更新和回滚

3. **React Router** - 路由管理
   - URL 级别的导航
   - 浏览器前进/后退
   - 深度链接支持

---

## 💻 开发指南

### 环境配置

**要求:**
- Node.js >= 18.0.0
- pnpm >= 8.0.0 （推荐） / npm / yarn

**安装依赖:**
```bash
cd web
pnpm install  # 或 npm install
```

**环境变量 (`.env`):**
```bash
VITE_BASE_API=/api
```

### 开发命令

- `src/login.tsx`
  - 提供用户名/密码登录表单
  - 使用 `authLogin`（定义在 `http.ts`）调用后端 `/auth/login`
  - 登录成功后将 token 持久化到 `localStorage`，并通过 `setLogined(true)` 通知 `App`
- `src/register.tsx`
  - 提供注册表单，调用 `/auth/register`
  - 注册成功后会给出成功提示，并短暂延时后自动退回登录页
- `src/App.tsx`
  - 维护以下关键状态：
    - `mode: "login" | "register"` – 当前显示登录还是注册表单
    - `logined: boolean` – 是否已登录
    - `loading: boolean` – 全局提交/请求中的遮罩
    - `usernameRef` – 当前登录用户名（传递给主面板）
  - 未登录时展示切换登录/注册的按钮和对应表单；登录成功后展示 `Mainpage` 主面板

### 2. 主控制台页面

- `src/mainpage.tsx`
  - 展示当前用户命名空间下的函数列表和选中函数详情
  - 依赖的接口：
    - `getFunctionsList` – 获取函数列表
    - `deployFunction` – 部署新函数
    - `updateFunction` – 更新函数配置
    - `deleteFunction` – 删除函数
    - `invokeFunction` – 调用函数
  - 内部状态：
    - `functions: FunctionItem[]` – 函数列表
    - `selFuncId: string | null` – 当前选中的函数名
    - `showDeployForm: boolean` – 是否显示“部署函数”表单
    - `form` – 部署/更新函数使用的表单模型（函数名 / namespace / 镜像）
  - 主要布局：
    - 顶部 Header：显示系统标题与当前用户组件 `User`
    - 左侧 Sidebar：
      - 「部署函数」按钮，打开 `Form` 部署表单
      - 「刷新列表」按钮，重新调用 `getFunctionsList`
      - 函数列表，使用 `FunctionItem` 按钮列表展示，可点击选中
    - 右侧主内容：
      - 若无函数：显示引导卡片，提示用户先部署函数
      - 若有函数但未选中：显示提示文案
      - 若选中函数：渲染 `FunctionInfo` 展示详细信息和操作

### 3. 函数详情与操作

- `src/function.tsx`
  - `FunctionItem`：左侧列表的每一项，负责视觉样式与选中态
  - `FunctionInfo`：主内容区的函数详情卡片，整合「查看/更新/删除/调用」等操作
  - 接收自 `Mainpage` 的属性：
    - 函数基础信息：`functionName`, `namespace`, `image`
    - 操作函数：`invokeFunction`, `deleteFunction`, `updateFunction`, `fetchList`, `setFunctions`
  - 内部状态：
    - `showUpdateForm` – 是否显示更新表单 `Form`
    - `submitting` – 更新表单的提交状态
    - `invokeForm` – 调用函数表单配置（路由、Header、Body 数据等）
    - `invokeSubmitting` – 调用请求的提交状态
    - `invokeResponse` – 最近一次调用的响应结果，传给 `Output` 组件展示
    - `showInvokeForm` – 是否显示调用参数配置弹窗 `InvokeForm`
  - 关键行为：
    - **删除函数**：使用 `useDebounce` 包裹 `deleteFunction`，删除成功后通过 `setFunctions` 从列表中移除该项
    - **更新函数**：点击「更新」时回填当前函数信息到 `form` 中并显示 `Form`，提交后调用 `updateFunction`
    - **调用函数**：点击「调用函数」时打开 `InvokeForm`，配置完整调用参数后通过 `invokeFunction` 发送请求

### 4. 表单与输出

- `src/form.tsx`
  - 封装部署/更新函数的通用表单 `Form` 与函数调用参数表单 `InvokeForm`
  - 使用 `formType` 区分部署(`deploy`)和更新(`update`)场景
  - 内聚对后端接口的调用逻辑，并在成功后触发 `fetchList()` 刷新列表
- `src/output.tsx`
  - 用于展示函数调用的结构化返回结果（一般是 JSON / 文本），与 `invokeResponse` 状态配合

### 5. HTTP 封装

- `src/http.ts`
  - 使用 `axios.create` 创建 `service` 实例，统一配置：
    - `baseURL`：来自 `VITE_BASE_API` 环境变量
    - `timeout`：10 秒
  - 请求拦截：
    - 自动附加 `Authorization: Bearer <token>` 到有 token 的请求
    - 为 GET 请求增加 `_t` 时间戳参数，避免缓存
  - 响应拦截：
    - 统一返回 `response.data`
    - 同时在浏览器环境触发一个 `window` 级别的 `CustomEvent("http:response")`，便于做全局调试或日志
  - 暴露的业务函数：
    - `authLogin`, `authRegister`
    - `getFunctionsList`, `deployFunction`, `deleteFunction`, `updateFunction`, `invokeFunction`

## 本次重写与优化内容

> 注：代码中已有的 `.backup` 目录保留为历史版本；当前 `src` 下文件为重写后的主版本。

### 1. 统一的页面流转

- 在 `App.tsx` 中：
  - 明确使用 `mode` 控制「登录/注册」展示，而不是通过多个条件层层嵌套
  - 全局的 `loading` 状态下显示遮罩层，避免用户在请求期间重复操作
  - 登录成功后通过 `usernameRef` 将用户名交给 `Mainpage`，作为 namespace 默认值

### 2. 函数管理视图的结构化

- 在 `mainpage.tsx` 中：
  - 使用 `useMemo` 计算当前选中的函数，避免在 `render` 中重复查找
  - 把获取列表 `fetchList` 和打开部署表单 `openDeploy` 抽成函数，逻辑更清晰
  - 增加空列表和未选中时的提示卡片，提升可用性
  - 左侧列表使用 `FunctionItem` 组件，右侧详情使用 `FunctionInfo`，实现职责清晰的双栏布局

### 3. `FunctionInfo`/`FunctionItem` 重写（当前文件 `function.tsx`）

- 将函数详情区域拆分为：
  - 顶部基本信息：函数名、namespace Badge、镜像信息
  - 中部操作按钮区：调用 / 更新 / 删除
  - 下部输出区：调用结果 `Output`
- 删除了无意义的 `console.log` 调用，只保留必要的错误输出和调试信息
- 使用项目已有的 UI 组件（`Button`, `Card`, `Badge`, `Separator` 等）统一视觉风格
- 删除操作增加 `useDebounce` 保护，避免用户快速连点导致重复请求或异常
- 将调用参数配置通过 `InvokeForm` 管理，并用 `invokeResponse` 与 `Output` 解耦

### 4. 接口层增强与类型整理

- 在 `http.ts` 中补充了接口返回的类型定义：
  - `AuthResponse`
  - `FunctionPayload`, `FunctionItem`
  - `InvokeHeader`
- 请求/响应拦截器内增加了必要的容错与日志，便于后续排查问题

## 开发与运行

### 本地开发

在仓库根目录下（或 `web` 目录），先安装依赖：

```bash
cd web
pnpm install  # 或 npm install / yarn
```

启动开发服务器：

```bash
pnpm dev
```

默认会在浏览器打开 `http://localhost:5173`（或 Vite 配置中指定的端口）。

### 构建与预览

```bash
pnpm build
pnpm preview
```

## 后续可以考虑的改进方向

- 引入路由：使用 `react-router` 等实现 URL 级别的路由（如 `/login`、`/functions/:name`），便于分享链接和浏览器前进/后退
- 错误与通知统一：将错误提示和成功提示统一到全局 Toast/Notification 系统，而不是在每个页面内单独渲染
- 表单校验增强：结合 `zod` 或 `react-hook-form` 做更严格的参数校验与错误提示
- 国际化：当前文案为中文，如有需要可接入简单的 i18n 方案

如果你希望，我也可以继续：
- 为所有表单与列表补充单元测试/组件测试
- 引入更完整的状态管理（如 Zustand/Recoil），避免多层组件传递回调
- 基于 OpenAPI (`docs/openapi.yaml`) 自动生成类型安全的 API 客户端。