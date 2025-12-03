import { useState } from "react";
import { Form, InvokeForm } from "./form";
import { Output } from "./output";
import { useDebounce } from "./debounce";
import { FunctionItem as FunctionItemType, FunctionPayload } from "./http";

interface FunctionItemProps {
  functionName: string;
  onClick: () => void;
  id?: string;
}

export function FunctionItem({ functionName, onClick }: FunctionItemProps) {
  return (
    <div>
      <button
        className="w-16 shadow-md rounded-md "
        onClick={() => {
          onClick();
        }}
      >
        {functionName}
      </button>
    </div>
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
      Content_Type: "",
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
    console.log("deleting function with payload:", payload);
    await deleteFunction(payload);
    setFunctions((prev) =>
      prev.filter(
        (f) => !(f.functionName === functionName && f.namespace === namespace),
      ),
    );
  }, 500);

  const handleInvoke = async () => {
    console.log(invokeForm);
    setShowInvokeForm(true);
  };
  
  return (
    <div>
      <p>Function: {functionName}</p>
      <p>Namespace: {namespace}</p>
      <p>Image: {image}</p>
      <button className="px-6 py-3 bg-gradient-to-r from-cyan-500 to-blue-500 text-white rounded-full shadow-lg hover:shadow-xl active:scale-95 transition" onClick={() => handleInvoke()}>Invoke</button>
      <button onClick={() => handleDelete(functionName, namespace)}>
        Delete
      </button>
      <button
        onClick={() => {
          openUpdate();
        }}
      >
        Update
      </button>
      <Output response={invokeResponse}> </Output>
      {/*不能直接传函数名进去，会把event直接传给函数*/}
      <div>
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
      </div>
      <div>
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
    </div>
  );
}
