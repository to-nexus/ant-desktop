import { useState } from "react";
import { type PendingConnectView } from "../lib/tauri";

interface ConnectConfirmModalProps {
  pending: PendingConnectView;
  onApprove: () => Promise<void>;
  onReject: () => Promise<void>;
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <span className="text-xs text-neutral-500 shrink-0">{label}</span>
      <span className="text-sm text-neutral-200 font-mono text-right break-all">
        {value}
      </span>
    </div>
  );
}

/**
 * Approval prompt for a deep-link connect.
 *
 * Names the account, not just the server. A link built by an attacker can point
 * at the real Ant server while carrying the attacker's own token — "connect to
 * ant.crosstoken.io?" is a question the user cannot answer wrongly, so the
 * account (and any switch away from the current one) is the actual decision.
 */
function ConnectConfirmModal({
  pending,
  onApprove,
  onReject,
}: ConnectConfirmModalProps) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async (action: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4">
      <div className="w-full max-w-md rounded-lg border border-neutral-800 bg-neutral-900 p-4 space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-neutral-500">
          Connection Request
        </h3>

        <p className="text-sm text-neutral-300">
          Something asked this app to connect to an Ant server and account.
          Approve only if you just started this sign-in yourself.
        </p>

        <div className="rounded-md border border-neutral-800 bg-neutral-950 p-3 space-y-2">
          <Row label="Server" value={pending.server} />
          <Row label="Account" value={pending.account ?? "unknown"} />
          {pending.currentAccount && (
            <Row label="Currently" value={pending.currentAccount} />
          )}
        </div>

        {pending.isAccountSwitch && (
          <p className="text-[11px] text-amber-400">
            ⚠️ This is a <strong>different account</strong> than the one this app
            is connected to. Approving hands your local Figma access to that
            account instead.
          </p>
        )}

        {!pending.account && (
          <p className="text-[11px] text-amber-400">
            ⚠️ The token does not say which account it belongs to.
          </p>
        )}

        {error && <p className="text-[11px] text-red-400">{error}</p>}

        <div className="flex justify-end gap-2 pt-1">
          <button
            onClick={() => run(onReject)}
            disabled={busy}
            className="text-xs px-3 py-1 bg-neutral-800 hover:bg-neutral-700 disabled:text-neutral-500 text-neutral-200 rounded transition-colors"
          >
            Reject
          </button>
          <button
            onClick={() => run(onApprove)}
            disabled={busy}
            className="text-xs px-3 py-1 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-700 disabled:text-neutral-500 text-white rounded transition-colors"
          >
            {busy ? "Connecting..." : "Approve"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default ConnectConfirmModal;
