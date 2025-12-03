import React, { useState, FormEvent, ChangeEvent } from "react";
import { authLogin } from "./http";

interface LoginProps {
  loading: boolean;
  setLoading: (value: boolean) => void;
  setLogined: (value: boolean) => void;
  usernameRef: React.MutableRefObject<string>;
}

function Login({ loading, setLoading, setLogined, usernameRef }: LoginProps) {
  const [username, setUsername] = useState<string>("");
  const [password, setPassword] = useState<string>("");
  const [error, setError] = useState<string>("");

  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError("");
    if (!username.trim() || !password.trim()) {
      setError("账号和密码不能为空");
      return;
    }

    setLoading(true);
    try {
      const body = await authLogin({ username, password });
      const token = body?.token || body?.access_token;
      if (token) {
        localStorage.setItem("token", token);
        setLogined(true);
        usernameRef.current = username;
        alert("登录成功");
      } else {
        setError(body?.message || JSON.stringify(body));
      }
    } catch (err: any) {
      const msg =
        err?.response?.data?.message ??
        err.message ??
        "网络错误";
      setError(msg);
      console.error("login error", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      style={{ maxWidth: 360, margin: "1rem auto", textAlign: "left" }}
    >
      <h1>Login</h1>

      {error && <div style={{ color: "red", marginBottom: 8 }}>{error}</div>}

      <div style={{ marginBottom: 8 }}>
        <label>
          Username:
          <input
            type="text"
            value={username}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setUsername(e.target.value)}
            style={{ width: "100%" }}
          />
        </label>
      </div>

      <div style={{ marginBottom: 12 }}>
        <label>
          Password:
          <input
            type="password"
            value={password}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setPassword(e.target.value)}
            style={{ width: "100%" }}
          />
        </label>
      </div>

      <button type="submit" disabled={loading}>
        {loading ? "Logging in..." : "Login"}
      </button>
    </form>
  );
}

export default Login;
