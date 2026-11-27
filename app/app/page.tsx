import { Hero } from "@/components/landing/Hero";
import { Steps } from "@/components/landing/Steps";
import { Payouts } from "@/components/landing/Payouts";
import { Dealer } from "@/components/landing/Dealer";
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
      </main>
    </>
  );
}
