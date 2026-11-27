import type { Metadata, Viewport } from "next";
import "./globals.css";
import "@/styles/tokens.css";
import "@/styles/shell.css";
import { MockStrip } from "@/components/shell/MockStrip";
import { TopBar } from "@/components/shell/TopBar";
import { Footer } from "@/components/shell/Footer";

export const metadata: Metadata = {
  title: "Jhandi Munda Casino - devnet",
  description: "The street dice game Jhandi Munda, onchain on Solana devnet.",
};

export const viewport: Viewport = { width: "device-width", initialScale: 1 };

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en">
      <body>
        <MockStrip />
        <TopBar />
        {children}
        <Footer />
      </body>
    </html>
  );
}
