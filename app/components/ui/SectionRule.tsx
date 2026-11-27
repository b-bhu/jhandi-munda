/** "01 How a round works ———": the numbered rule that opens each landing section. */
export function SectionRule({ n, children }: { n: string; children: React.ReactNode }) {
  return <div className="rule"><span className="l"><b>{n}</b> {children}</span><span className="x" /></div>;
}
