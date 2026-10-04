import { useEffect, useRef } from "react";

/** Enkel bekreftelsesdialog med `<dialog>`, så fokus og Escape håndteres av nettleseren. */
export function ConfirmDialog({
  open,
  title,
  text,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  text: string;
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (open && !d.open) d.showModal?.();
    if (!open && d.open) d.close?.();
  }, [open]);
  if (!open) return null;
  return (
    <dialog ref={ref} className="dialog" aria-labelledby="dialog-tittel" onCancel={onCancel} open>
      <h2 id="dialog-tittel" className="dialog__title">
        {title}
      </h2>
      <p>{text}</p>
      <div className="dialog__actions">
        <button type="button" className="btn btn--secondary" onClick={onCancel} autoFocus>
          {cancelLabel}
        </button>
        <button type="button" className="btn btn--danger" onClick={onConfirm}>
          {confirmLabel}
        </button>
      </div>
    </dialog>
  );
}
