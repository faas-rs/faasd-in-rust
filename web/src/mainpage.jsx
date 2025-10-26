import {useEffect, useState} from 'react';
import { getFunctionsList, 
  deployFunction, deleteFunction, 
  updateFunction, invokeFunction } from './http.js';
import { Form } from './deploy.jsx';
import { FunctionItem } from './function.jsx';

function Mainpage ({username}) {

  const [functions, setFunctions] = useState([]);
  const [showDeployForm, setShowDeployForm] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [form, setForm] = useState({
    functionName: '',
    namespace: username,
    image: '',
  });

  useEffect(()=>{
    fetchList();
  },[])

  const openDeploy = () => {
    setForm({ functionName: '', namespace: username, image: '' });
    setShowDeployForm(true);
  };
  // 从后端获取响应并写入state
  const fetchList = async () => {
    try {
      const response = await getFunctionsList({ namespace: username });
      console.log('raw response from getFunctionsList:', response);
      const list = Array.isArray(response) 
      ? response.map(item =>{
        return {
          functionName: item.functionName || '',
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
          <FunctionItem id = {func.functionName} key={func.functionName} 
          {...func}
            updateFunction={updateFunction}
            fetchList={fetchList}
            deleteFunction={deleteFunction}
            invokeFunction={invokeFunction}
            setFunctions={setFunctions}
           />
        ))}
      </div>
      {showDeployForm && (
        <Form submitting={submitting}
          setSubmitting={setSubmitting}
          setShowForm={setShowDeployForm}
          form={form}
          setForm={setForm}
          deployFunction={deployFunction}
          fetchList={fetchList}
          updateFunction={updateFunction}
          formType = 'deploy'></Form>
      )}
    </div>    
  )
}

export default Mainpage;  