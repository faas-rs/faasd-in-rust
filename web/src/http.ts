import axios, { AxiosInstance, AxiosError, InternalAxiosRequestConfig, AxiosResponse } from "axios";

const service: AxiosInstance = axios.create({
  baseURL: import.meta.env.VITE_BASE_API as string,
  timeout: 10 * 1000,
});

service.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    const token = localStorage.getItem("token");
    if (token && config.headers) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    // 防止 GET 缓存
    if (config.method === "get") {
      config.params = { ...config.params, _t: Date.now() };
    }
    return config;
  },
  (error: AxiosError) => Promise.reject(error),
);

service.interceptors.response.use(
  (response: AxiosResponse) => {
    const { data } = response;
    try {
      window.dispatchEvent(
        new CustomEvent("http:response", {
          detail: {
            url: response.config?.url,
            status: response.status,
            data,
          },
        }),
      );
    } catch (e) {
      // ignore in non-browser env
    }
    return data;
  },
  (error: AxiosError) => {
    console.error("deploy failed", error);
    return Promise.reject(error);
  },
);

// 定义 API 返回类型
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

// 业务接口
export const authLogin = (payload: { username: string; password: string }): Promise<AuthResponse> =>
  axios
    .post(`${import.meta.env.VITE_BASE_API || ""}/auth/login`, payload)
    .then((res) => res.data);

export const authRegister = (payload: { username: string; password: string }): Promise<AuthResponse> =>
  axios
    .post(`${import.meta.env.VITE_BASE_API || ""}/auth/register`, payload)
    .then((res) => res.data);

export const getFunctionsList = (params: Record<string, any> = {}): Promise<FunctionItem[]> =>
  service.get("/system/functions", { params });

export const deployFunction = (payload: FunctionPayload): Promise<any> =>
  service.post("/system/functions", payload);

export const deleteFunction = (payload: Pick<FunctionPayload, 'functionName' | 'namespace'>): Promise<any> =>
  service.delete(`/system/functions`, { data: payload });

export const updateFunction = (payload: FunctionPayload): Promise<any> =>
  service.put(`/system/functions`, payload);

export const invokeFunction = (
  functionName: string,
  namespace: string,
  route: string,
  data: any,
  contentType: string,
): Promise<any> =>
  service.post(`/function/${functionName}.${namespace}${route}`, data, {
    headers: { "Content-Type": contentType },
  });

export default service;
