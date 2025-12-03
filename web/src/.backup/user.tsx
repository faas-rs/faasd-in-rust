import React from "react";

interface UserProps {
  username: React.MutableRefObject<string>;
  setlogined: (value: boolean) => void;
  className?: string;
}

export default function User({ username, setlogined, className }: UserProps) {
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
