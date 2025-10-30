import { useEffect, useMemo, useState } from "react";
import {
  getFunctionsList,
  deployFunction,
  deleteFunction,
  updateFunction,
  invokeFunction,
} from "./http.js";
import { Form } from "./form.jsx";
import { FunctionItem, FunctionInfo } from "./function.jsx";

function Mainpage({ username }) {
  const [functions, setFunctions] = useState([]);
  const [showDeployForm, setShowDeployForm] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [selFuncId, setSelFuncId] = useState(null);
  const [form, setForm] = useState({
    functionName: "",
    namespace: username,
    image: "",
  });

  const selFunc = useMemo(() => {
    return functions.find((f) => f.functionName === selFuncId) || null;
  }, [functions, selFuncId]);

  useEffect(() => {
    fetchList();
  }, []);

  const openDeploy = () => {
    setForm({ functionName: "", namespace: username, image: "" });
    setShowDeployForm(true);
  };
  // 从后端获取响应并写入state
  const fetchList = async () => {
    try {
      const response = await getFunctionsList({ namespace: username });
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
    }
  };

  return (
    <div className="flex h-screen">
      <div className="w-64 flex flex-col gap-3 h-screen p-4">
        <h1>{username}</h1>
        <div className="flex gap-3">
          <button className="w-16 shadow-md rounded-md " onClick={openDeploy}>
            deploy
          </button>
          <button className="w-16 shadow-md rounded-md " onClick={fetchList}>
            getlist
          </button>
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
          ></Form>
        )}

        <div className="flex flex-col gap-3">
          <div>functions:</div>
          {functions.map((func) => (
            <FunctionItem
              id={func.functionName}
              key={func.functionName}
              functionName={func.functionName}
              onClick={() => {
                if (selFuncId === func.functionName) {
                  setSelFuncId(null);
                } else {
                  setSelFuncId(func.functionName);
                }
              }}
            />
          ))}
        </div>
      </div>
      {
        <div className="flex-1 overflow-y-auto p-6">
          {selFunc && (
            <FunctionInfo
              {...selFunc}
              invokeFunction={invokeFunction}
              deleteFunction={deleteFunction}
              updateFunction={updateFunction}
              fetchList={fetchList}
              setFunctions={setFunctions}
            />
          )}
        </div>
      }
    </div>
  );
}

export default Mainpage;
