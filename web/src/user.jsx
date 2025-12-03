export default function User({ username, setlogined, className }) {
  const logout = () => {
    console.log("logging out");
    localStorage.removeItem("token");
    username.current = "";
    setlogined(false);
  };
  return (
    <div className={className}>
      <div>{username.current}</div>
      <button
        onClick={() => {
          logout();
        }}
      >
        logout
      </button>
    </div>
  );
}
