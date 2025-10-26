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
    try {
      window.dispatchEvent(new CustomEvent('http:response', {
        detail: {
          url: response.config?.url,
          status: response.status,
          data,
        }
      }))
    } catch (e) {
      // ignore in non-browser env
    }
    return data   
  },
  error => {
    const msg = error.response?.data?.msg || error.message || '网络错误'
    console.error('deploy failed', error);
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

export const getFunctionsList = (params = {}) => service.get('/system/functions', {params})
export const deployFunction = (payload) => service.post('/system/functions', payload)
export const deleteFunction = (payload) => service.delete(`/system/functions`,
{data: payload})
export const updateFunction = (payload) => 
  service.put(`/system/functions`, payload)
export const invokeFunction = (functionName, namespace) => 
  service.post(`/function/${functionName}.${namespace}/${functionName}`)
// export const someApi = (data) => service.post('/some', data)

export default service