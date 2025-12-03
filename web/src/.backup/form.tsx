import { FormEvent, ChangeEvent } from "react";
import { FunctionPayload } from "./http";

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
  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setForm((s) => ({ ...s, [name]: value }));
  };
  
  const handleSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setSubmitting(true);
    try {
      // 根据你的后端接口调整 payload 结构
      const payload: FunctionPayload = {
        functionName: form.functionName,
        namespace: form.namespace,
        image: form.image,
      };
      if (formType === "deploy") {
        await deployFunction(payload);
      } else if (formType === "update") {
        await updateFunction(payload);
      }
      setShowForm(false);
      await fetchList();
    } catch (err) {
      console.error("error", err);
      // 可在这里显示错误提示
    } finally {
      setSubmitting(false);
    }
  };
  
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.45)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9999,
      }}
    >
      <form
        onSubmit={handleSubmit}
        style={{
          background: "#fff",
          padding: 20,
          borderRadius: 8,
          width: 420,
          boxShadow: "0 6px 30px rgba(0,0,0,0.3)",
          display: "flex",
          flexDirection: "column",
          gap: 10,
        }}
      >
        {formType === "deploy" && <h3>Deploy Function</h3>}
        {formType === "update" && <h3>Update Function</h3>}

        <label>
          FunctionName
          <input
            name="functionName"
            value={form.functionName}
            onChange={handleChange}
            required
          />
        </label>

        <label>
          Image
          <input
            name="image"
            value={form.image}
            onChange={handleChange}
            required
          />
        </label>

        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button
            type="button"
            onClick={() => setShowForm(false)}
            disabled={submitting}
          >
            Cancel
          </button>
          <button type="submit" disabled={submitting}>
            {submitting ? "Submitting..." : "Deploy"}
          </button>
        </div>
      </form>
    </div>
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
      setShowForm(false);
      console.log("invoke response:", response);
    } catch (error) {
      console.log("err", error);
    } finally {
      setSubmitting(false);
    }
  };
  
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.45)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9999,
      }}
    >
      <form
        onSubmit={handleSubmit}
        style={{
          background: "#fff",
          padding: 20,
          borderRadius: 8,
          width: 420,
          boxShadow: "0 6px 30px rgba(0,0,0,0.3)",
          display: "flex",
          flexDirection: "column",
          gap: 10,
        }}
      >
        <h3>Invoke function</h3>
        <label>
          route
          <input
            name="route"
            value={form.route}
            onChange={handleChange}
            required
          />
        </label>

        <label>
          Content-Type
          <input
            value={form.header.Content_Type}
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setForm((s) => ({
                ...s,
                header: { ...s.header, Content_Type: e.target.value },
              }))
            }
            required
          />
        </label>
        <label>
          data
          <input
            name="data"
            value={form.data}
            onChange={handleChange}
            required
          />
        </label>

        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button
            type="button"
            onClick={() => setShowForm(false)}
            disabled={submitting}
          >
            Cancel
          </button>
          <button type="submit" disabled={submitting}>
            {submitting ? "Submitting..." : "Invoke"}
          </button>
        </div>
      </form>
    </div>
  );
}
