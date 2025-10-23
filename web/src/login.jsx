import React ,{useState, useRef}from 'react';
function Login() {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = (e) => {
    e.preventDefault();
    if (!username.trim() || !password.trim()) {
      setError('账号和密码不能为空');
      return;
    }
    setError('');
    setLoading(true);
  };
  const loginRequest = async () => {
    const res = await fetch ('https://http://127.0.0.1:8080/auth/login')
  }
  return (
    <form onSubmit={handleSubmit}>
      <div>
        <label htmlFor="username">Username:</label>
        <input 
            type="text" 
            id="username" 
            placeholder='username'
            value={username}
            onChange={(e) => setUsername(e.target.value)}>
        </input>
      </div>
      <div>
        <label htmlFor="password">Password:</label>
        <input 
            type="password" 
            id="password" 
            placeholder='password'
            value={password}
            onChange={(e) => setPassword(e.target.value)}>
        </input>
      </div>
      <button type="submit">Login</button>
    </form>
  );
}

export default Login;
