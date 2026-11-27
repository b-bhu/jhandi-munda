import type { SymbolName } from "@/lib/symbols";

/* The six board symbols, drawn as in docs/design (red hearts and diamonds, dark spade, club and flag,
   gold crown). The colours are fixed because they sit on the cloth and on the dice, never on the dark page. */
const PATHS: Record<SymbolName, React.ReactNode> = {
  heart: <path fill="#b3382c" d="M12 21s-8.5-5.4-8.5-11.4A4.6 4.6 0 0 1 12 7.2a4.6 4.6 0 0 1 8.5 2.4C20.5 15.600 12 21 12 21z" />,
  spade: <path fill="#141414" d="M12 2C8 8 3.500 10 3.500 14.200a4 4 0 0 0 7 2.600c0 2-.6 3.400-2 4.700h7c-1.400-1.300-2-2.700-2-4.700a4 4 0 0 0 7-2.600C20.500 10 16 8 12 2z" />,
  diamond: <path fill="#b3382c" d="M12 2l9 10-9 10-9-10z" />,
  club: <g fill="#141414"><circle cx="12" cy="7" r="4.200" /><circle cx="7" cy="14" r="4.200" /><circle cx="17" cy="14" r="4.200" /><path d="M12 12l2.500 9.500h-5z" /></g>,
  crown: <><path fill="#c9971f" stroke="#7a5a10" strokeWidth=".6" d="M2.500 18.500L4 7l5 5 3-7 3 7 5-5 1.500 11.500z" /><rect x="2.500" y="19" width="19" height="2.200" fill="#a67c15" /></>,
  flag: <><rect x="5" y="2" width="1.800" height="20" fill="#2a2a2a" /><path fill="#141414" d="M7 3.500c4-1.500 6 1.500 11 0l-2 4.500 2.500 4.500c-5 1.500-7-1.500-11.500 0z" /></>,
};

/** `size` sets a fixed pixel size; leave it out when CSS sizes the icon (tiles, dice). */
export function SymbolIcon({ name, size }: { name: SymbolName; size?: number }) {
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} role="img" aria-label={name}>
      {PATHS[name]}
    </svg>
  );
}
