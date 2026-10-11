import { useEffect } from "react";
import { useLauncher } from "../store";

export function Toast() {
  const toast = useLauncher((s) => s.toast);
  const closeToast = useLauncher((s) => s.closeToast);

  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(closeToast, 2600);
    return () => clearTimeout(t);
  }, [toast, closeToast]);

  if (!toast) return null;
  return (
    <div className="toast" onClick={closeToast}>
      {toast}
    </div>
  );
}
