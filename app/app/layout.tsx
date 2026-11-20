import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Jhandi Munda Casino - devnet",
  description: "The street dice game Jhandi Munda, onchain on Solana devnet.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
