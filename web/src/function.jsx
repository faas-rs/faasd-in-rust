import { useState } from "react";
import { Form } from "./deploy.jsx";
export function FunctionItem({ functionName, image, 
    namespace ,deleteFunction,
    invokeFunction,updateFunction,fetchList,setFunctions}) {
    const [showInfo, setShowInfo] = useState(false);
    return (
        <div>
            <button onClick={() => setShowInfo(!showInfo)}>{functionName}</button>
            {showInfo && <FunctionInfo 
            invokeFunction={invokeFunction}
            deleteFunction={deleteFunction}
            updateFunction={updateFunction}
            fetchList={fetchList}
            functionName={functionName}
            namespace={namespace}
            image={image}
            setFunctions={setFunctions}
            />}
        </div>
    );
}
function FunctionInfo({
    invokeFunction,
    deleteFunction,
    updateFunction,
    fetchList,
    functionName,
    namespace,
    image,
    setFunctions
}) {
    const [showUpdateForm,setShowUpdateForm] = useState(false);
    const [submitting,setSubmitting] = useState(false);
    const [form,setForm] = useState({
        functionName: functionName,
        namespace: namespace,
        image: image,
    });
    function openUpdate(){
        setForm({
            functionName: functionName,
            namespace: namespace,
            image: image})
        setShowUpdateForm(true);
    }

    const handleDelete = async (functionName, namespace) => {
        const payload = { 
            functionName:form.functionName, 
            namespace:form.namespace} ;
        await deleteFunction(payload);
        setFunctions(prev => prev.filter(f => !(f.functionName === functionName && f.namespace === namespace)));
    }
    return (
        <div>
            <p>Function: {functionName}</p>
            <p>Namespace: {namespace}</p>
            <p>Image: {image}</p>
            <button onClick={() => invokeFunction(functionName, namespace)}>Invoke</button>
            <button onClick={() => handleDelete(functionName, namespace)}>Delete</button>
            <button onClick={() => {openUpdate()}}>Update</button>{/*不能直接传函数名进去，会把event直接传给函数*/}
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
        </div>
    );
}