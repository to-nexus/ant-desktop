import { type ReactNode } from "react";

interface ConnectionCardProps {
  title: string;
  children: ReactNode;
}

function ConnectionCard({ title, children }: ConnectionCardProps) {
  return (
    <div className="rounded-lg border border-neutral-800 bg-neutral-900 p-4">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-neutral-500 mb-3">
        {title}
      </h3>
      {children}
    </div>
  );
}

export default ConnectionCard;
