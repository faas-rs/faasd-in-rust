import axios from 'axios'
import { ElMessage } from 'element-plus'   

// 1. 创建 axios 实例，统一基准地址、超时时间
const service = axios.create({
  baseURL: import.meta.env.VITE_BASE_API, 

  timeout: 10 * 1000
})

// 2. 请求拦截：每次自动携带 token / 加时间戳
service.interceptors.request.use(
  (config) => {
    const token = localStorage.getItem('token')
    if (token) config.headers.Authorization = `Bearer ${token}`
    // 防止 GET 缓存
    if (config.method === 'get') {
      config.params = { ...config.params, _t: Date.now() }
    }
    return config
  },
  error => Promise.reject(error)
)

// 3. 响应拦截：统一弹错、统一解构数据
service.interceptors.response.use(
  (response) => {
    const { data } = response
    // 后端约定：code === 0 是成功；其他都是错
    if (data.code !== 0) {
      ElMessage.error(data.msg || '服务器异常')
      return Promise.reject(new Error(data.msg || 'Error'))
    }
    return data.data   // 只把真正的业务数据抛给调用方
  },
  error => {
    // HTTP 状态码非 2xx
    const msg = error.response?.data?.msg || error.message || '网络错误'
    ElMessage.error(msg)
    return Promise.reject(error)
  }
)

// 4. 业务接口——这里集中写
export const getUser = id => service.get(`/user/${id}`)
export const getUserList = params => service.get('/user', { params })
export const addUser = data => service.post('/user', data)
export const updateUser = (id, data) => service.put(`/user/${id}`, data)
export const delUser = id => service.delete(`/user/${id}`)

// 订单模块
export const addOrder = data => service.post('/order', data)
export const getOrder = id => service.get(`/order/${id}`)

// 文件上传
export const upload = file => {
  const fd = new FormData()
  fd.append('file', file)
  return service.post('/upload', fd, {
    headers: { 'Content-Type': 'multipart/form-data' }
  })
}

// 默认导出实例，万一哪里需要特殊配置可直接用
export default service