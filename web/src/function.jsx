import { useState ,useCallback} from "react";
import { Form, InvokeForm } from "./form.jsx";
import { Output } from "./output.jsx";
import { useDebounce } from "./debounce.jsx";

export function FunctionItem({ functionName, onClick }) {
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

export function FunctionInfo({
  invokeFunction,
  deleteFunction,
  updateFunction,
  fetchList,
  functionName,
  namespace,
  image,
  setFunctions,
}) {
  const [showUpdateForm, setShowUpdateForm] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [response, setResponse] = useState(null);
  const [form, setForm] = useState({
    functionName: functionName,
    namespace: namespace,
    image: image,
  });
  const [invokeForm, setInvokeForm] = useState({
    route: "",
    header: {
      Content_Type: "",
      /**/
    },
    data: "",
  });
  const [invokeSubmitting, setInvokeSubmitting] = useState(false);
  const [invokeResponse, setInvokeResponse] = useState("");
  const [showInvokeForm, setShowInvokeForm] = useState(false);

  function openUpdate() {
    setForm({
      functionName: functionName,
      namespace: namespace,
      image: image,
    });
    setShowUpdateForm(true);
  }

  const handleDelete = useDebounce(async (functionName, namespace) => {
    const payload = { 
      functionName: functionName,
      namespace: namespace
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
      <button onClick={() => handleInvoke()}>Invoke</button>
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
