import { useState, FormEvent, ChangeEvent } from "react";
import { authRegister } from "./http";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { UserPlus, Loader2 } from "lucide-react";
import { useToast } from "@/hooks/use-toast";
import { extractErrorMessage } from "./types";

interface RegisterProps {
  loading: boolean;
  setLoading: (value: boolean) => void;
  onCancel?: () => void;
}

function Register({ loading, setLoading, onCancel }: RegisterProps) {
  const [username, setUsername] = useState<string>("");
  const [password, setPassword] = useState<string>("");
  const [error, setError] = useState<string>("");
  const [success, setSuccess] = useState<string>("");
  const { toast } = useToast();

  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setError("");
    setSuccess("");
    if (!username.trim() || !password.trim()) {
      setError("用户名和密码不能为空");
      return;
    }

    setLoading(true);
    try {
      const body = await authRegister({ username, password });
      if (body && (body.user_id || body.message)) {
        const successMsg = body.message || "注册成功！请登录";
        setSuccess(successMsg);
        toast({
          title: "注册成功",
          description: successMsg,
          variant: "success",
        });
        setTimeout(() => {
          onCancel && onCancel();
        }, 1500);
      } else {
        const errMsg = "注册失败，请重试";
        setError(errMsg);
        toast({
          title: "注册失败",
          description: errMsg,
          variant: "destructive",
        });
      }
    } catch (err: any) {
      const msg = extractErrorMessage(err);
      setError(msg);
      toast({
        title: "注册错误",
        description: msg,
        variant: "destructive",
      });
      console.error("register error", err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Card className="w-full max-w-md">
      <CardHeader className="space-y-1">
        <CardTitle className="text-2xl font-bold">注册</CardTitle>
        <CardDescription>创建一个新账户来使用 Faasd</CardDescription>
      </CardHeader>
      <form onSubmit={handleSubmit}>
        <CardContent className="space-y-4">
          {error && (
            <div className="p-3 text-sm text-destructive bg-destructive/10 border border-destructive/20 rounded-md">
              {error}
            </div>
          )}
          {success && (
            <div className="p-3 text-sm text-green-700 bg-green-50 border border-green-200 rounded-md">
              {success}
            </div>
          )}
          <div className="space-y-2">
            <Label htmlFor="reg-username">用户名</Label>
            <Input
              id="reg-username"
              type="text"
              placeholder="选择一个用户名"
              value={username}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setUsername(e.target.value)}
              disabled={loading}
              autoComplete="username"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="reg-password">密码</Label>
            <Input
              id="reg-password"
              type="password"
              placeholder="选择一个密码"
              value={password}
              onChange={(e: ChangeEvent<HTMLInputElement>) => setPassword(e.target.value)}
              disabled={loading}
              autoComplete="new-password"
            />
          </div>
        </CardContent>
        <CardFooter className="flex gap-2">
          <Button type="button" variant="outline" className="flex-1" onClick={onCancel} disabled={loading}>
            取消
          </Button>
          <Button type="submit" className="flex-1" disabled={loading}>
            {loading ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                注册中...
              </>
            ) : (
              <>
                <UserPlus className="mr-2 h-4 w-4" />
                注册
              </>
            )}
          </Button>
        </CardFooter>
      </form>
    </Card>
  );
}

export default Register;
