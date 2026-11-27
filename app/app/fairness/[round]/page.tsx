import { Suspense } from "react";
import { FairnessClient } from "@/components/fairness/FairnessClient";

export const metadata = { title: "Fairness · Jhandi Munda" };

async function Round({ params }: { params: Promise<{ round: string }> }) {
  const { round } = await params;
  const id = Number(round);
  if (!Number.isInteger(id) || id < 0) return <div className="page"><h1>Unknown round</h1><p>Round numbers are whole numbers, like 12.</p></div>;
  return <FairnessClient roundId={id} />;
}

export default function FairnessPage(props: PageProps<"/fairness/[round]">) {
  return (
    <Suspense fallback={<div className="page">Loading the proof…</div>}>
      <Round params={props.params} />
    </Suspense>
  );
}
