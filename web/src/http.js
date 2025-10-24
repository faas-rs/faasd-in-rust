import axios from 'axios'
 
const service = axios.create({
  baseURL: import.meta.env.VITE_BASE_API, 

  timeout: 10 * 1000
})


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


service.interceptors.response.use(
  (response) => {
    const { data } = response
    if (data.code !== 0) {
      ElMessage.error(data.msg || '服务器异常')
      return Promise.reject(new Error(data.msg || 'Error'))
    }
    return data.data   
  },
  error => {
    const msg = error.response?.data?.msg || error.message || '网络错误'
    ElMessage.error(msg)
    return Promise.reject(error)
  }
)

// 4. 业务接口——这里集中写
export const authLogin = (payload) =>
  axios.post(`${import.meta.env.VITE_BASE_API || ''}/auth/login`, payload)
    .then(res => res.data)

// 新增：注册接口，同样返回后端 body 由组件处理
export const authRegister = (payload) =>
  axios.post(`${import.meta.env.VITE_BASE_API || ''}/auth/register`, payload)
    .then(res => res.data)

export const getFunctionsList = () => service.get('/system/functions')
export const deployFunction = (payload) => service.post('/system/functions', payload)
export const deleteFunction = (functionName, namespace) => service.delete(`/system/functions`,
  { params: { function_name: functionName, namespace } })
export const updateFunction = (functionName, payload) => 
  service.put(`/system/functions`, payload)
export const invokeFunction = (functionName, namespace) => 
  service.post(`/function/${functionName}_${namespace}/${functionName}`)
// export const someApi = (data) => service.post('/some', data)

export default service