import { Button } from "@/components/ui/button";
import { LogOut, User as UserIcon } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import type { UserInfo } from "./types";

interface UserProps {
  userInfo: UserInfo;
  setlogined: (value: boolean) => void;
  className?: string;
}

export default function User({ userInfo, setlogined, className }: UserProps) {
  const logout = () => {
    console.log("logging out");
    localStorage.removeItem("token");
    setlogined(false);
  };

  return (
    <div className={className}>
      <Dialog>
        <DialogTrigger asChild>
          <Button variant="ghost" className="gap-2">
            <UserIcon className="h-4 w-4" />
            <span className="hidden sm:inline">{userInfo.username}</span>
          </Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>确认退出</DialogTitle>
            <DialogDescription>
              你确定要退出吗？退出后需要重新登录才能访问。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => {}}>取消</Button>
            <Button variant="destructive" onClick={logout}>
              <LogOut className="mr-2 h-4 w-4" />
              退出登录
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
