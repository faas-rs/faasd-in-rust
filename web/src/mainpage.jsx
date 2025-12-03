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
import User from "./user.jsx";

function Mainpage({ username, setLogined }) {
  const [functions, setFunctions] = useState([]);
  const [showDeployForm, setShowDeployForm] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [selFuncId, setSelFuncId] = useState(null);
  const [form, setForm] = useState({
    functionName: "",
    namespace: username.current,
    image: "",
  });

  const selFunc = useMemo(() => {
    return functions.find((f) => f.functionName === selFuncId) || null;
  }, [functions, selFuncId]);

  useEffect(() => {
    fetchList();
  }, []);

  const openDeploy = () => {
    setForm({ functionName: "", namespace: username.current, image: "" });
    setShowDeployForm(true);
  };
  // 从后端获取响应并写入state
  const fetchList = async () => {
    try {
      const response = await getFunctionsList({ namespace: username.current });
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
    <div className="min-h-screen flex flex-col">
      <header className="shrink-0 h-16 bg-indigo-600 text-white flex items-center px-6 shadow">
        <h1>fassd-in-rust</h1>
        <User
          className="flex gap-3 fixed right-5"
          username={username}
          setlogined={setLogined}
        />
      </header>
      <div className="flex-1 flex min-h-0">
        <aside className="w-64 shrink-0 bg-gray-50 border-r border-gray-200 flex flex-col">
          <div className="p-4 flex gap-3 border-b border-gray-200">
            <button className="w-16 shadow-md rounded-md" onClick={openDeploy}>
              deploy
            </button>
            <button className="w-16 shadow-md rounded-md" onClick={fetchList}>
              getlist
            </button>
          </div>
          {showDeployForm && (
            <div className="p-4 border-b border-gray-200">
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
            </div>
          )}
          <div className="flex-1 overflow-y-auto p-4">
            <div className="text-sm font-medium text-gray-600 mb-2">
              functions:
            </div>
            <div className="flex flex-col gap-2">
              {functions.map((func) => (
                <FunctionItem
                  id={func.functionName}
                  key={func.functionName}
                  functionName={func.functionName}
                  onClick={() =>
                    setSelFuncId(
                      selFuncId === func.functionName
                        ? null
                        : func.functionName,
                    )
                  }
                />
              ))}
            </div>
          </div>
        </aside>
        <main className="flex-1 overflow-y-auto p-6 bg-white">
          {functions.length === 0 ? (
            <div className="text-gray-400 text-center mt-20">
              请部署一个函数
            </div>
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
            <div className="text-gray-400 text-center mt-20">
              请从左侧选择一个函数
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

export default Mainpage;
