import {useEffect, useState} from 'react';
import { getFunctionsList, 
  deployFunction, deleteFunction, 
  updateFunction, invokeFunction } from './http.js';
import { Deployform } from './deploy.jsx';
import { FunctionItem } from './function.jsx';

function Mainpage ({username}) {

  const [functions, setFunctions] = useState([]);
  const [showDeployForm, setShowDeployForm] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [form, setForm] = useState({
    functionName: '',
    namespace: '',
    image: '',
  });

  useEffect(()=>{
    fetchList();
  },[])

  const openDeploy = () => {
    setForm({ functionName: '', namespace: '', image: '' });
    setShowDeployForm(true);
  };
  // 从后端获取响应并写入state
  const fetchList = async () => {
    try {
      const response = await getFunctionsList();
      const list = Array.isArray(response) 
      ? response.map(item =>{
        return {
          functionName: item.function_name || '',
          namespace: item.namespace || '',
          image: item.image || '',
        }
      }):[]
      setFunctions(list);
    } catch (err) {
      console.error('fetchList error', err);
    }
  };

  return (
    <div>
      <h1>{username}</h1>
      <button key = 'deploy' onClick={openDeploy}>deploy</button>
      <button key = 'getlist' onClick={fetchList}>getlist</button>
      <div>
        {functions.map(func => (
          <FunctionItem key={func.functionName} {...func} />
        ))}
      </div>
      {showDeployForm && (
        <Deployform submitting={submitting}
          setSubmitting={setSubmitting}
          setShowDeployForm={setShowDeployForm}
          form={form}
          setForm={setForm}
          deployFunction={deployFunction}
          fetchList={fetchList}></Deployform>
      )}
    </div>    
  )
}

export default Mainpage;  