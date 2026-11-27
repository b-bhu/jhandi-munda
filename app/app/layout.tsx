import type { Metadata, Viewport } from "next";
import { Instrument_Serif, Manrope, Inter_Tight, JetBrains_Mono } from "next/font/google";
import "./globals.css";
import "@/styles/tokens.css";
import "@/styles/shell.css";
import { MockStrip } from "@/components/shell/MockStrip";
import { TopBar } from "@/components/shell/TopBar";
import { Footer } from "@/components/shell/Footer";

const serif = Instrument_Serif({ subsets: ["latin"], weight: "400", style: ["normal", "italic"], variable: "--font-serif", display: "swap" });
const ui = Manrope({ subsets: ["latin"], weight: ["400", "500", "600", "700"], variable: "--font-ui", display: "swap" });
const sans = Inter_Tight({ subsets: ["latin"], weight: ["400", "500", "600"], variable: "--font-sans", display: "swap" });
const mono = JetBrains_Mono({ subsets: ["latin"], weight: ["400", "500", "600", "700"], variable: "--font-mono", display: "swap" });

export const metadata: Metadata = {
  title: "Jhandi Munda Casino - devnet",
  description: "The street dice game Jhandi Munda, onchain on Solana devnet.",
};

export const viewport: Viewport = { width: "device-width", initialScale: 1 };

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={`${serif.variable} ${ui.variable} ${sans.variable} ${mono.variable}`}>
      <body>
        <a className="skip" href="#main">Skip to content</a>
        <MockStrip />
        <TopBar />
        <main id="main">{children}</main>
        <Footer />
      </body>
    </html>
  );
}
