import {useEffect, useState} from 'react';
import { getFunctionsList, 
  deployFunction, deleteFunction, 
  updateFunction, invokeFunction } from './http.js';
import { Deployform } from './deploy.jsx';

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

  const fetchList = async () => {
    try {
      const list = await getFunctionsList();
      setFunctions(Array.isArray(list) ? list : []);
    } catch (err) {
      console.error('fetchList error', err);
    }
  };

  return (
    <div>
      <h1>{username}</h1>
      <button key = 'deploy' onClick={openDeploy}>deploy</button>
      <button key = 'getlist' onClick={fetchList}>getlist</button>

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