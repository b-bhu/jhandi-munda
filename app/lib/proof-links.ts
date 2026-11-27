/*
 * Real devnet transactions shown on the landing page. Copied by script from docs/deploys.md
 * (27 Nov 2026 deploy and the round 12 proof), not typed by hand. Open them on Solana Explorer.
 */
export interface ProofLink { label: string; sig: string }

export const LANDING_PROOFS: readonly ProofLink[] = [
  { label: "Program upgrade on devnet", sig: "xCb37j2DoQrEP5uJMdJWchRF9FtCJuft89DhLhV6gA3XdmhMHPFxJZRdxwskJFbvuXaZatTaktkMGpNhSDY41pt" },
  { label: "Config upgrade (74 to 91 bytes)", sig: "4udnHt1FXXriZB4o2V6c9swRvVasPYA5843gL1eeSi6NmiDEanUind6PnPzAUHc3pGfJFdPF1ENkuekFLUFKLUPg" },
  { label: "Dice roll from VRF, round 12", sig: "5aQ75QwPrQafMvV8sbHgZW69afP1fH4N3HaB2tm9jJAZBH83Turikxv1bmL3iq2s4CvmJ8V2Ma8iMap1aXhyAZtF" },
  { label: "Queued win paid out", sig: "5yX32eVCaGaYvXV2nWPxcjM4wAP7egB7d5GAzq8dgxafh9pm64DpkQasJHe1ouuVxrEnMHaNMuMUCsyiqVrUfJLS" },
];
