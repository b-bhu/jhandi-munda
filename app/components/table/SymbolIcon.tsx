import type { SymbolName } from "@/lib/symbols";

/* Six simple board symbols as inline SVG on a 48 x 48 box. They use currentColor, so a tile can tint them. */
const PATHS: Record<SymbolName, React.ReactNode> = {
  heart: <path d="M24 42 6 24C-1 16 6 4 16 6c4 1 7 4 8 7 1-3 4-6 8-7 10-2 17 10 10 18z" />,
  spade: <path d="M24 4C14 16 5 22 5 30c0 6 5 9 10 8 3-1 5-2 6-4-1 5-3 8-6 10h18c-3-2-5-5-6-10 1 2 3 3 6 4 5 1 10-2 10-8 0-8-9-14-19-26z" />,
  diamond: <path d="M24 3 40 24 24 45 8 24z" />,
  club: <path d="M24 5a8 8 0 0 0-6 13 8 8 0 1 0 5 14c-1 5-3 8-6 11h14c-3-3-5-6-6-11a8 8 0 1 0 5-14 8 8 0 0 0-6-13z" />,
  crown: <path d="M5 38 8 14l10 11 6-14 6 14 10-11 3 24zM7 41h34v3H7z" />,
  flag: <path d="M10 4h3v40h-3zM15 6h26l-7 9 7 9H15z" />,
};

export function SymbolIcon({ name, size = 40 }: { name: SymbolName; size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 48 48" fill="currentColor" role="img" aria-label={name}>
      {PATHS[name]}
    </svg>
  );
}
