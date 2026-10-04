export function Toast({ message }: { message: string | null }) {
  return (
    <div className={message ? "toast toast--on" : "toast"} role="status" aria-live="polite">
      {message}
    </div>
  );
}
