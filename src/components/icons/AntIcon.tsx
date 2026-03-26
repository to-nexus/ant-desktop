interface AntIconProps {
  size?: number;
  className?: string;
}

export default function AntIcon({ size = 24, className }: AntIconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      <defs>
        <linearGradient id="antGrad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#60a5fa" />
          <stop offset="100%" stopColor="#a78bfa" />
        </linearGradient>
      </defs>
      <circle cx="16" cy="16" r="4" fill="url(#antGrad)" />
      <circle cx="8" cy="8" r="2.5" fill="#60a5fa" opacity={0.8} />
      <circle cx="24" cy="8" r="2.5" fill="#60a5fa" opacity={0.8} />
      <circle cx="8" cy="24" r="2.5" fill="#a78bfa" opacity={0.8} />
      <circle cx="24" cy="24" r="2.5" fill="#a78bfa" opacity={0.8} />
      <path d="M9.5 9.5 L12.5 12.5" stroke="#60a5fa" strokeWidth="1.5" strokeLinecap="round" opacity={0.4} />
      <path d="M22.5 9.5 L19.5 12.5" stroke="#60a5fa" strokeWidth="1.5" strokeLinecap="round" opacity={0.4} />
      <path d="M9.5 22.5 L12.5 19.5" stroke="#a78bfa" strokeWidth="1.5" strokeLinecap="round" opacity={0.4} />
      <path d="M22.5 22.5 L19.5 19.5" stroke="#a78bfa" strokeWidth="1.5" strokeLinecap="round" opacity={0.4} />
      <circle cx="16" cy="16" r="6" stroke="#93c5fd" strokeWidth="0.5" opacity={0.2} fill="none" />
    </svg>
  );
}
