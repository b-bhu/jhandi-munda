/*
 * The six board symbols. The program only knows the numbers 1 to 6 (a die face is
 * `(byte % 6) + 1`, a bet symbol must be in 1..=6). The NAMES below are a UI convention,
 * taken from the board's layout in docs/characters/README.md: top row heart, spade, diamond,
 * bottom row club, crown, flag. Nothing onchain fixes them, so this file is the one place to change.
 */
export type SymbolId = 1 | 2 | 3 | 4 | 5 | 6;
export type SymbolName = "heart" | "spade" | "diamond" | "club" | "crown" | "flag";

export const SYMBOLS: readonly { id: SymbolId; name: SymbolName; label: string }[] = [
  { id: 1, name: "heart", label: "Heart" },
  { id: 2, name: "spade", label: "Spade" },
  { id: 3, name: "diamond", label: "Diamond" },
  { id: 4, name: "club", label: "Club" },
  { id: 5, name: "crown", label: "Crown (Munda)" },
  { id: 6, name: "flag", label: "Flag (Jhandi)" },
];

export function symbolName(id: number): SymbolName {
  return SYMBOLS.find((s) => s.id === id)?.name ?? "heart";
}
