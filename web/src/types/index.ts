// API 相关类型
export interface AuthResponse {
  token?: string;
  access_token?: string;
  message?: string;
  user_id?: string;
  msg?: string;
}

export interface FunctionPayload {
  functionName: string;
  namespace: string;
  image?: string;
}

export interface FunctionItem {
  functionName: string;
  namespace: string;
  image: string;
}

export interface InvokeHeader {
  Content_Type?: string;
  [key: string]: string | undefined;
}

export interface InvokePayload {
  route: string;
  header: {
    Content_Type: string;
  };
  data: string;
}

// 组件 Props 类型
export interface UserInfo {
  username: string;
  namespace?: string;
}

// 表单类型
export interface DeployFormData {
  functionName: string;
  namespace: string;
  image: string;
}

export interface InvokeFormData {
  route: string;
  header: {
    Content_Type: string;
  };
  data: string;
}

// 常量
export const DEFAULT_CONTENT_TYPE = "application/json";
export const DEFAULT_NAMESPACE = "default";

// 错误处理
export interface ApiError {
  message: string;
  code?: string;
  status?: number;
}

export function extractErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  
  if (typeof error === "object" && error !== null) {
    const err = error as any;
    return (
      err?.response?.data?.message ||
      err?.response?.data?.msg ||
      err?.message ||
      "未知错误"
    );
  }
  
  return "操作失败";
}
