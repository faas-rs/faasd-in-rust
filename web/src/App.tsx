import { useState } from "react";
import Login from "./login";
import Register from "./register";
import Mainpage from "./mainpage";
import { Button } from "@/components/ui/button";
import { Toaster } from "@/components/ui/toaster";
import { Loader2 } from "lucide-react";
import type { UserInfo } from "./types";

type Mode = "login" | "register";

function App() {
  const [loading, setLoading] = useState<boolean>(false);
  const [logined, setLogined] = useState<boolean>(false);
  const [mode, setMode] = useState<Mode>("login");
  const [userInfo, setUserInfo] = useState<UserInfo>({ username: "" });

  if (logined) {
    return (
      <>
        <Mainpage userInfo={userInfo} setLogined={setLogined} />
        <Toaster />
      </>
    );
  }

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-4 bg-gradient-to-br from-blue-50 via-indigo-50 to-purple-50">
      <div className="absolute top-8 left-8 flex items-center gap-4">
        <h1 className="text-3xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-blue-600 to-indigo-600">
          Faasd in Rust
        </h1>
        <span className="text-xs text-muted-foreground bg-white px-2 py-1 rounded-full border">
          v0.1.0
        </span>
      </div>

      <div className="w-full max-w-md space-y-4">
        <div className="flex gap-2 justify-center">
          <Button
            variant={mode === "login" ? "default" : "outline"}
            onClick={() => setMode("login")}
            disabled={mode === "login"}
          >
            登录
          </Button>
          <Button
            variant={mode === "register" ? "default" : "outline"}
            onClick={() => setMode("register")}
            disabled={mode === "register"}
          >
            注册
          </Button>
        </div>

        {mode === "login" ? (
          <Login
            loading={loading}
            setLoading={setLoading}
            setLogined={setLogined}
            setUserInfo={setUserInfo}
          />
        ) : (
          <Register
            loading={loading}
            setLoading={setLoading}
            onCancel={() => setMode("login")}
          />
        )}
      </div>

      {loading && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-background p-6 rounded-lg shadow-lg flex items-center gap-3">
            <Loader2 className="h-5 w-5 animate-spin" />
            <span>处理中...</span>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
