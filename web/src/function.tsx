import { useState } from "react";
import { Form, InvokeForm } from "./form";
import { Output } from "./output";
import { useDebounce } from "./debounce";
import { FunctionItem as FunctionItemType, FunctionPayload } from "./http";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Play, Trash2, Edit, Package } from "lucide-react";
import { Separator } from "@/components/ui/separator";
import { useToast } from "@/hooks/use-toast";
import { extractErrorMessage } from "./types";

interface FunctionItemProps {
  functionName: string;
  onClick: () => void;
  id?: string;
  isSelected?: boolean;
}

export function FunctionItem({ functionName, onClick, isSelected }: FunctionItemProps) {
  return (
    <button
      onClick={onClick}
      className={`w-full text-left px-3 py-2 rounded-md transition-colors ${
        isSelected
          ? "bg-primary text-primary-foreground"
          : "hover:bg-accent hover:text-accent-foreground"
      }`}
    >
      <div className="flex items-center gap-2">
        <Package className="h-4 w-4" />
        <span className="text-sm font-medium truncate">{functionName}</span>
      </div>
    </button>
  );
}

interface FunctionInfoProps extends FunctionItemType {
  invokeFunction: (
    functionName: string,
    namespace: string,
    route: string,
    data: any,
    contentType: string,
  ) => Promise<any>;
  deleteFunction: (payload: Pick<FunctionPayload, 'functionName' | 'namespace'>) => Promise<any>;
  updateFunction: (payload: FunctionPayload) => Promise<any>;
  fetchList: () => Promise<void>;
  setFunctions: React.Dispatch<React.SetStateAction<FunctionItemType[]>>;
}

export function FunctionInfo({
  invokeFunction,
  deleteFunction,
  updateFunction,
  fetchList,
  functionName,
  namespace,
  image,
  setFunctions,
}: FunctionInfoProps) {
  const { toast } = useToast();
  const [showUpdateForm, setShowUpdateForm] = useState<boolean>(false);
  const [submitting, setSubmitting] = useState<boolean>(false);
  const [form, setForm] = useState({
    functionName: functionName,
    namespace: namespace,
    image: image,
  });
  const [invokeForm, setInvokeForm] = useState({
    route: "",
    header: {
      Content_Type: "application/json",
    },
    data: "",
  });
  const [invokeSubmitting, setInvokeSubmitting] = useState<boolean>(false);
  const [invokeResponse, setInvokeResponse] = useState<string>("");
  const [showInvokeForm, setShowInvokeForm] = useState<boolean>(false);

  function openUpdate() {
    setForm({
      functionName: functionName,
      namespace: namespace,
      image: image,
    });
    setShowUpdateForm(true);
  }

  const handleDelete = useDebounce(async (functionName: string, namespace: string) => {
    const payload = {
      functionName: functionName,
      namespace: namespace,
    };
    try {
      console.log("deleting function with payload:", payload);
      await deleteFunction(payload);
      setFunctions((prev) =>
        prev.filter(
          (f) => !(f.functionName === functionName && f.namespace === namespace),
        ),
      );
      toast({
        title: "删除成功",
        description: `函数 ${functionName} 已被删除`,
        variant: "success",
      });
    } catch (err) {
      console.error("delete error", err);
      toast({
        title: "删除失败",
        description: extractErrorMessage(err),
        variant: "destructive",
      });
    }
  }, 500);

  const handleInvoke = async () => {
    setShowInvokeForm(true);
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <div className="flex items-start justify-between">
            <div>
              <CardTitle className="text-2xl">{functionName}</CardTitle>
              <CardDescription className="mt-2">
                <Badge variant="secondary">{namespace}</Badge>
              </CardDescription>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div>
            <h3 className="text-sm font-medium text-muted-foreground mb-1">镜像</h3>
            <code className="text-sm bg-muted px-2 py-1 rounded">{image}</code>
          </div>

          <Separator />

          <div className="flex flex-wrap gap-2">
            <Button onClick={() => handleInvoke()} className="gap-2">
              <Play className="h-4 w-4" />
              调用函数
            </Button>
            <Button variant="outline" onClick={() => openUpdate()} className="gap-2">
              <Edit className="h-4 w-4" />
              更新
            </Button>
            <Button
              variant="destructive"
              onClick={() => handleDelete(functionName, namespace)}
              className="gap-2"
            >
              <Trash2 className="h-4 w-4" />
              删除
            </Button>
          </div>
        </CardContent>
      </Card>

      <Output response={invokeResponse} />

      {showUpdateForm && (
        <Form
          submitting={submitting}
          setSubmitting={setSubmitting}
          setShowForm={setShowUpdateForm}
          form={form}
          setForm={setForm}
          deployFunction={updateFunction}
          fetchList={fetchList}
          updateFunction={updateFunction}
          formType="update"
        />
      )}

      {showInvokeForm && (
        <InvokeForm
          functionName={functionName}
          namespace={namespace}
          submitting={invokeSubmitting}
          setSubmitting={setInvokeSubmitting}
          setShowForm={setShowInvokeForm}
          form={invokeForm}
          setForm={setInvokeForm}
          invokeFunction={invokeFunction}
          invokeResponse={invokeResponse}
          setInvokeResponse={setInvokeResponse}
        />
      )}
    </div>
  );
}
