import type { ReactNode } from "react";

/** The raised box with two corner brackets (`.box` in tokens.css). */
export function CornerBox({ children, className = "", style }: { children: ReactNode; className?: string; style?: React.CSSProperties }) {
  return <div className={`box ${className}`.trim()} style={style}>{children}</div>;
}
