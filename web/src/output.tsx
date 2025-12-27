import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Code, Copy, Check } from "lucide-react";
import { useToast } from "@/hooks/use-toast";

interface OutputProps {
  response: string;
  children?: React.ReactNode;
}

export function Output({ response }: OutputProps) {
  const [copied, setCopied] = useState(false);
  const { toast } = useToast();

  if (!response) return null;

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(response);
      setCopied(true);
      toast({
        title: "已复制",
        description: "响应内容已复制到剪贴板",
        variant: "success",
      });
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      toast({
        title: "复制失败",
        description: "无法复制到剪贴板",
        variant: "destructive",
      });
    }
  };

  // 尝试格式化 JSON
  let formattedResponse = response;
  let isJson = false;
  try {
    const parsed = JSON.parse(response);
    formattedResponse = JSON.stringify(parsed, null, 2);
    isJson = true;
  } catch {
    // 不是有效的 JSON，使用原始响应
  }

  return (
    <Card className="mt-4">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg flex items-center gap-2">
            <Code className="h-5 w-5" />
            调用响应
            {isJson && (
              <span className="text-xs font-normal text-muted-foreground">(JSON)</span>
            )}
          </CardTitle>
          <Button
            variant="outline"
            size="sm"
            onClick={handleCopy}
            className="gap-2"
          >
            {copied ? (
              <>
                <Check className="h-3 w-3" />
                已复制
              </>
            ) : (
              <>
                <Copy className="h-3 w-3" />
                复制
              </>
            )}
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        <pre className="bg-muted p-4 rounded-md overflow-x-auto text-sm max-h-[500px] overflow-y-auto">
          <code className={isJson ? "language-json" : ""}>{formattedResponse}</code>
        </pre>
      </CardContent>
    </Card>
  );
}
