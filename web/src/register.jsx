import React, { useState } from 'react'
import { authRegister } from './http.js'

function Register({ loading, setLoading, setLogined, onCancel }) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')

  const handleSubmit = async (e) => {
    e.preventDefault()
    setError('')
    if (!username.trim() || !password.trim()) {
      setError('账号和密码不能为空')
      return
    }

    setLoading(true)
    try {
      const body = await authRegister({ username, password })
      if (body && (body.user_id || body.message)) {
        alert(body.message || '注册成功，请登录')
        onCancel && onCancel()
      } else {
        setError(JSON.stringify(body))
      }
    } catch (err) {
        const status = err?.response?.status
        const msg = err?.response?.data?.msg 
        ?? err?.response?.data?.message 
        ?? err.message 
        ?? '网络错误';
        setError(msg);
        console.error('register error', err);
    } finally {
      setLoading(false)
    }
  }

  return (
    <form onSubmit={handleSubmit} style={{ maxWidth: 360, margin: '1rem auto', textAlign: 'left' }}>
      <h1>Register</h1>

      {error && <div style={{ color: 'red', marginBottom: 8 }}>{error}</div>}

      <div style={{ marginBottom: 8 }}>
        <label>
          Username:
          <input type="text" value={username} onChange={(e) => setUsername(e.target.value)} style={{ width: '100%' }} />
        </label>
      </div>

      <div style={{ marginBottom: 12 }}>
        <label>
          Password:
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} style={{ width: '100%' }} />
        </label>
      </div>

      <div style={{ display: 'flex', gap: 8 }}>
        <button type="submit" disabled={loading}>{loading ? 'Registering...' : 'Register'}</button>
        <button type="button" onClick={onCancel} disabled={loading}>Cancel</button>
      </div>
    </form>
  )
}

export default Register
