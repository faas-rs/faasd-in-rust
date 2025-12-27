import { useEffect, useMemo, useState } from "react";
import {
  getFunctionsList,
  deployFunction,
  deleteFunction,
  updateFunction,
  invokeFunction,
  FunctionItem as FunctionItemType,
} from "./http";
import { Form } from "./form";
import { FunctionItem, FunctionInfo } from "./function";
import User from "./user";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Upload, RefreshCw, ServerCog } from "lucide-react";
import { useToast } from "@/hooks/use-toast";
import { extractErrorMessage, type UserInfo } from "./types";

interface MainpageProps {
  userInfo: UserInfo;
  setLogined: (value: boolean) => void;
}

function Mainpage({ userInfo, setLogined }: MainpageProps) {
  const [functions, setFunctions] = useState<FunctionItemType[]>([]);
  const [showDeployForm, setShowDeployForm] = useState<boolean>(false);
  const [submitting, setSubmitting] = useState<boolean>(false);
  const [selFuncId, setSelFuncId] = useState<string | null>(null);
  const [form, setForm] = useState({
    functionName: "",
    namespace: userInfo.namespace || userInfo.username,
    image: "",
  });
  const { toast } = useToast();

  const selFunc = useMemo(() => {
    return functions.find((f) => f.functionName === selFuncId) || null;
  }, [functions, selFuncId]);

  useEffect(() => {
    fetchList();
  }, []);

  const openDeploy = () => {
    setForm({ 
      functionName: "", 
      namespace: userInfo.namespace || userInfo.username, 
      image: "" 
    });
    setShowDeployForm(true);
  };

  const fetchList = async () => {
    try {
      const response = await getFunctionsList({ 
        namespace: userInfo.namespace || userInfo.username 
      });
      console.log("raw response from getFunctionsList:", response);
      const list = Array.isArray(response)
        ? response.map((item) => {
            return {
              functionName: item.functionName || "",
              namespace: item.namespace || "",
              image: item.image || "",
            };
          })
        : [];
      setFunctions(list);
    } catch (err) {
      console.error("fetchList error", err);
      toast({
        title: "获取函数列表失败",
        description: extractErrorMessage(err),
        variant: "destructive",
      });
    }
  };

  const isEmpty = Array.isArray(functions) && functions.length === 0;

  return (
    <div className="min-h-screen flex flex-col bg-background">
      {/* Header */}
      <header className="sticky top-0 z-40 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-16 items-center justify-between px-6">
          <div className="flex items-center gap-2">
            <ServerCog className="h-6 w-6 text-primary" />
            <h1 className="text-xl font-bold">Faasd 管理面板</h1>
          </div>
          <User userInfo={userInfo} setlogined={setLogined} />
        </div>
      </header>

      <div className="flex flex-1">
        {/* Sidebar */}
        <aside className="w-72 border-r bg-muted/40">
          <div className="flex flex-col h-full">
            <div className="p-4 space-y-2 border-b">
              <Button onClick={openDeploy} className="w-full gap-2">
                <Upload className="h-4 w-4" />
                部署函数
              </Button>
              <Button onClick={fetchList} variant="outline" className="w-full gap-2">
                <RefreshCw className="h-4 w-4" />
                刷新列表
              </Button>
            </div>

            {showDeployForm && (
              <Form
                submitting={submitting}
                setSubmitting={setSubmitting}
                setShowForm={setShowDeployForm}
                form={form}
                setForm={setForm}
                deployFunction={deployFunction}
                fetchList={fetchList}
                updateFunction={updateFunction}
                formType="deploy"
              />
            )}

            <div className="flex-1 overflow-hidden">
              <div className="p-4">
                <h3 className="text-sm font-medium text-muted-foreground mb-3">
                  函数列表 ({functions.length})
                </h3>
                <ScrollArea className="h-[calc(100vh-280px)]">
                  <div className="space-y-1">
                    {functions.map((func) => (
                      <FunctionItem
                        id={func.functionName}
                        key={func.functionName}
                        functionName={func.functionName}
                        isSelected={selFuncId === func.functionName}
                        onClick={() =>
                          setSelFuncId(
                            selFuncId === func.functionName ? null : func.functionName,
                          )
                        }
                      />
                    ))}
                  </div>
                </ScrollArea>
              </div>
            </div>
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 overflow-auto">
          <div className="container max-w-5xl py-6 px-6">
            {isEmpty ? (
              <Card className="p-12 text-center">
                <ServerCog className="h-16 w-16 mx-auto text-muted-foreground mb-4" />
                <h3 className="text-lg font-medium mb-2">暂无函数</h3>
                <p className="text-muted-foreground mb-4">
                  点击左侧"部署函数"按钮创建你的第一个函数
                </p>
                <Button onClick={openDeploy} className="gap-2">
                  <Upload className="h-4 w-4" />
                  部署函数
                </Button>
              </Card>
            ) : selFunc ? (
              <FunctionInfo
                {...selFunc}
                invokeFunction={invokeFunction}
                deleteFunction={deleteFunction}
                updateFunction={updateFunction}
                fetchList={fetchList}
                setFunctions={setFunctions}
              />
            ) : (
              <Card className="p-12 text-center">
                <h3 className="text-lg font-medium text-muted-foreground">
                  从左侧选择一个函数以查看详情
                </h3>
              </Card>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}

export default Mainpage;
