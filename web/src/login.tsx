import { useState, FormEvent, ChangeEvent } from "react";
import { authLogin } from "./http";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { LogIn, Loader2 } from "lucide-react";
import { useToast } from "@/hooks/use-toast";
import { extractErrorMessage, type UserInfo } from "./types";

interface LoginProps {
  loading: boolean;
  setLoading: (value: boolean) => void;
  setLogined: (value: boolean) => void;
  setUserInfo: (userInfo: UserInfo) => void;
}

function Login({ loading, setLoading, setLogined, setUserInfo }: LoginProps) {
  const [username, setUsername] = useState<string>("");
  const [password, setPassword] = useState<string>("");
  const [error, setError] = useState<string>("");
  const { toast } = useToast();

  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError("");
    if (!username.trim() || !password.trim()) {
      setError("用户名和密码不能为空");
      return;
    }

    setLoading(true);
    try {
      const body = await authLogin({ username, password });
      const token = body?.token || body?.access_token;
      if (token) {
        localStorage.setItem("token", token);
        setUserInfo({ username, namespace: username });
        setLogined(true);
        toast({
          title: "登录成功",
          description: `欢迎回来, ${username}`,
          variant: "success",
        });
      } else {
        const errMsg = body?.message || "登录失败，请重试";
        setError(errMsg);
        toast({
          title: "登录失败",
          description: errMsg,
          variant: "destructive",
        });
      }
    } catch (err: any) {
      const msg = extractErrorMessage(err);
      setError(msg);
      toast({
        title: "登录错误",
        description: msg,
        variant: "destructive",
      });
      console.error("login error", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card className="w-full max-w-md">
      <CardHeader className="space-y-1">
        <CardTitle className="text-2xl font-bold">登录</CardTitle>
        <CardDescription>输入你的凭据以访问 Faasd 管理面板</CardDescription>
      </CardHeader>
      <form onSubmit={handleSubmit}>
        <CardContent className="space-y-4">
          {error && (
            <div className="p-3 text-sm text-destructive bg-destructive/10 border border-destructive/20 rounded-md">
              {error}
            </div>
          )}
          <div className="space-y-2">
            <Label htmlFor="username">用户名</Label>
            <Input
              id="username"
              type="text"
              placeholder="输入用户名"
              value={username}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setUsername(e.target.value)}
              disabled={loading}
              autoComplete="username"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="password">密码</Label>
            <Input
              id="password"
              type="password"
              placeholder="输入密码"
              value={password}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setPassword(e.target.value)}
              disabled={loading}
              autoComplete="current-password"
            />
          </div>
        </CardContent>
        <CardFooter>
          <Button type="submit" className="w-full" disabled={loading}>
            {loading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                登录中...
              </>
            ) : (
              <>
                <LogIn className="mr-2 h-4 w-4" />
                登录
              </>
            )}
          </Button>
        </CardFooter>
      </form>
    </Card>
  );
}

export default Login;
