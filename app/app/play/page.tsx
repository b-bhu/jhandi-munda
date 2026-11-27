import { Suspense } from "react";
import { PlayClient } from "@/components/table/PlayClient";

export const metadata = { title: "Play · Jhandi Munda" };

export default function PlayPage() {
  return (
    <Suspense fallback={<div className="page">Loading the table…</div>}>
      <PlayClient />
    </Suspense>
  );
}
