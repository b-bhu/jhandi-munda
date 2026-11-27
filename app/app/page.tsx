import { Hero } from "@/components/landing/Hero";
import { Steps } from "@/components/landing/Steps";
import { Payouts } from "@/components/landing/Payouts";
import { Dealer } from "@/components/landing/Dealer";
import { WinLose } from "@/components/landing/WinLose";
import { House } from "@/components/landing/House";
import { Proof } from "@/components/landing/Proof";
import "@/styles/landing.css";

export const metadata = { title: "Jhandi Munda Casino · devnet" };

export default function Home() {
  return (
    <>
      <Hero />
      <main className="wrap">
        <Steps />
        <Payouts />
        <Dealer />
        <WinLose />
        <House />
        <Proof />
      </main>
    </>
  );
}
