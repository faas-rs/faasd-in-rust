import { useState , useRef} from 'react'
import Login from './login'
import Register from './register'
import Mainpage from './mainpage'

function App() {

  const [loading, setLoading] = useState(false);
  const [logined, setLogined] = useState(false);
  const [mode, setMode] = useState('login'); // 'login' | 'register'
  const usernameRef = useRef('defaultUser');

  if (logined) {
    return (
		<Mainpage username={usernameRef.current}> </Mainpage>
    )
  }

	return (
		<div className="App" style={{ padding: 20 }}>
			<h1>faasd-in-rs UI</h1>

			<div style={{ marginBottom: 12 }}>
        <button onClick={() => setMode('login')} disabled={mode === 'login'}>Login</button>
        <button onClick={() => setMode('register')} disabled={mode === 'register'}>Register</button>
      </div>

			{mode === 'login' ? (
        <Login loading={loading} setLoading={setLoading} setLogined={setLogined} usernameRef={usernameRef} />
      ) : (
        <Register loading={loading} setLoading={setLoading} setLogined={setLogined} onCancel={() => setMode('login')} />
      )}

			{loading && (
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
					<div
						style={{
							padding: '1rem 1.5rem',
							background: '#fff',
							borderRadius: 8,
							boxShadow: '0 4px 20px rgba(0,0,0,0.2)',
							display: 'flex',
							alignItems: 'center',
							gap: 12,
						}}
					>
						<div className="spinner" aria-hidden style={{ width: 18, height: 18, border: '3px solid #ccc', borderTop: '3px solid #333', borderRadius: '50%', animation: 'spin 1s linear infinite' }} />
						<span>Loading…</span>
					</div>
					<style>{`@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }`}</style>
				</div>
			)}
		</div>
	)
}

export default App
