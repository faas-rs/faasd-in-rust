interface OutputProps {
  response: string;
  children?: React.ReactNode;
}

export function Output({ response }: OutputProps) {
  return (
    <div>
      <h3>{response}</h3>
    </div>
  );
}
