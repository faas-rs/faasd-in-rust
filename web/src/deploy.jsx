export function Deployform({submitting, 
    setSubmitting, setShowDeployForm, form, setForm, 
    deployFunction, fetchList}){

    const handleChange = (e) => {
        const { name, value } = e.target;
        setForm((s) => ({ ...s, [name]: value }));
    };
    const handleSubmit = async (e) => {
        e.preventDefault();
        setSubmitting(true);
        try {
          // 根据你的后端接口调整 payload 结构
          const payload = {
            name: form.functionName,
            image: form.image,
            namespace: form.namespace,
          };
          await deployFunction(payload);
          setShowDeployForm(false);
          await fetchList();
        } catch (err) {
          console.error('deploy error', err);
          // 可在这里显示错误提示
        } finally {
          setSubmitting(false);
        }
    };
    return(
        <div
          style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(0,0,0,0.45)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 9999,
          }}
        >
          <form
            onSubmit={handleSubmit}
            style={{
              background: '#fff',
              padding: 20,
              borderRadius: 8,
              width: 420,
              boxShadow: '0 6px 30px rgba(0,0,0,0.3)',
              display: 'flex',
              flexDirection: 'column',
              gap: 10,
            }}
          >
            <h3>Deploy Function</h3>

            <label>
              FunctionName
              <input name="name" value={form.name} onChange={handleChange} required />
            </label>

            <label>
              Image
              <input name="image" value={form.image} onChange={handleChange} required />
            </label>

            <label>
              Namespace
              <input name="namespace" value={form.namespace} onChange={handleChange} required />
            </label>

            <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
              <button type="button" onClick={() => setShowDeployForm(false)} disabled={submitting}>
                Cancel
              </button>
              <button type="submit" disabled={submitting}>
                {submitting ? 'Submitting...' : 'Deploy'}
              </button>
            </div>
          </form>
        </div>
    )
}