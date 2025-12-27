import { FormEvent, ChangeEvent } from "react";
import { FunctionPayload } from "./http";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Loader2, Upload, Edit } from "lucide-react";
import { useToast } from "@/hooks/use-toast";
import { extractErrorMessage } from "./types";

interface FormProps {
  submitting: boolean;
  setSubmitting: (value: boolean) => void;
  setShowForm: (value: boolean) => void;
  form: {
    functionName: string;
    namespace: string;
    image: string;
  };
  setForm: React.Dispatch<React.SetStateAction<{
    functionName: string;
    namespace: string;
    image: string;
  }>>;
  deployFunction: (payload: FunctionPayload) => Promise<any>;
  fetchList: () => Promise<void>;
  updateFunction: (payload: FunctionPayload) => Promise<any>;
  formType: "deploy" | "update";
}

export function Form({
  submitting,
  setSubmitting,
  setShowForm,
  form,
  setForm,
  deployFunction,
  fetchList,
  updateFunction,
  formType,
}: FormProps) {
  const { toast } = useToast();
  
  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setForm((s) => ({ ...s, [name]: value }));
  };

  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setSubmitting(true);
    try {
      const payload: FunctionPayload = {
        functionName: form.functionName,
        namespace: form.namespace,
        image: form.image,
      };
      if (formType === "deploy") {
        await deployFunction(payload);
        toast({
          title: "部署成功",
          description: `函数 ${form.functionName} 已成功部署`,
          variant: "success",
        });
      } else if (formType === "update") {
        await updateFunction(payload);
        toast({
          title: "更新成功",
          description: `函数 ${form.functionName} 已成功更新`,
          variant: "success",
        });
      }
      setShowForm(false);
      await fetchList();
    } catch (err) {
      console.error("error", err);
      const action = formType === "deploy" ? "部署" : "更新";
      toast({
        title: `${action}失败`,
        description: extractErrorMessage(err),
        variant: "destructive",
      });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={true} onOpenChange={(open) => !open && setShowForm(false)}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {formType === "deploy" ? (
              <>
                <Upload className="h-5 w-5" />
                部署函数
              </>
            ) : (
              <>
                <Edit className="h-5 w-5" />
                更新函数
              </>
            )}
          </DialogTitle>
          <DialogDescription>
            {formType === "deploy" ? "部署一个新的函数到集群" : "更新现有函数的配置"}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="functionName">函数名称</Label>
              <Input
                id="functionName"
                name="functionName"
                value={form.functionName}
                onChange={handleChange}
                placeholder="my-function"
                required
                disabled={submitting}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="image">镜像地址</Label>
              <Input
                id="image"
                name="image"
                value={form.image}
                onChange={handleChange}
                placeholder="docker.io/library/hello-world:latest"
                required
                disabled={submitting}
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setShowForm(false)}
              disabled={submitting}
            >
              取消
            </Button>
            <Button type="submit" disabled={submitting}>
              {submitting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  提交中...
                </>
              ) : (
                formType === "deploy" ? "部署" : "更新"
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface InvokeFormProps {
  functionName: string;
  namespace: string;
  submitting: boolean;
  setSubmitting: (value: boolean) => void;
  setShowForm: (value: boolean) => void;
  form: {
    route: string;
    header: {
      Content_Type: string;
    };
    data: string;
  };
  setForm: React.Dispatch<React.SetStateAction<{
    route: string;
    header: {
      Content_Type: string;
    };
    data: string;
  }>>;
  invokeFunction: (
    functionName: string,
    namespace: string,
    route: string,
    data: any,
    contentType: string,
  ) => Promise<any>;
  invokeResponse: string;
  setInvokeResponse: (value: string) => void;
}

export function InvokeForm({
  functionName,
  namespace,
  submitting,
  setSubmitting,
  setShowForm,
  form,
  setForm,
  invokeFunction,
  setInvokeResponse,
}: InvokeFormProps) {
  const { toast } = useToast();
  
  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setForm((s) => ({ ...s, [name]: value }));
  };

  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setSubmitting(true);
    try {
      const route = form.route;
      const contentType = form.header.Content_Type;
      const data = form.data;
      const response = await invokeFunction(
        functionName,
        namespace,
        route,
        data,
        contentType,
      );
      setInvokeResponse(JSON.stringify(response, null, 2));
      toast({
        title: "调用成功",
        description: `函数 ${functionName} 已成功调用`,
        variant: "success",
      });
      setShowForm(false);
      console.log("invoke response:", response);
    } catch (error) {
      console.log("err", error);
      const errMsg = extractErrorMessage(error);
      setInvokeResponse("错误: " + errMsg);
      toast({
        title: "调用失败",
        description: errMsg,
        variant: "destructive",
      });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={true} onOpenChange={(open) => !open && setShowForm(false)}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>调用函数</DialogTitle>
          <DialogDescription>
            调用 {functionName}.{namespace}
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="route">路由</Label>
              <Input
                id="route"
                name="route"
                value={form.route}
                onChange={handleChange}
                placeholder="/api/endpoint"
                required
                disabled={submitting}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="contentType">Content-Type</Label>
              <Input
                id="contentType"
                value={form.header.Content_Type}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  setForm((s) => ({
                    ...s,
                    header: { ...s.header, Content_Type: e.target.value },
                  }))
                }
                placeholder="application/json"
                required
                disabled={submitting}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="data">请求数据 (JSON)</Label>
              <Textarea
                id="data"
                name="data"
                value={form.data}
                onChange={(e: ChangeEvent<HTMLTextAreaElement>) =>
                  setForm((s) => ({ ...s, data: e.target.value }))
                }
                placeholder='{"key": "value"}'
                required
                disabled={submitting}
                rows={6}
                className="font-mono text-sm"
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setShowForm(false)}
              disabled={submitting}
            >
              取消
            </Button>
            <Button type="submit" disabled={submitting}>
              {submitting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  调用中...
                </>
              ) : (
                "调用"
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
