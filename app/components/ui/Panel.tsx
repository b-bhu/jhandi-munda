import type { ReactNode } from "react";
import { ExampleTag } from "@/components/shell/ExampleTag";

/** A titled box. `example` adds the EXAMPLE tag (mock mode only); `tag` replaces it with another one. */
export function Panel({ title, example = false, tag, children, id }: { title?: string; example?: boolean; tag?: ReactNode; children: ReactNode; id?: string }) {
  return (
    <section className="panel" id={id}>
      {(title || example || tag) && (
        <div className="panel-head">
          {title ? <h2>{title}</h2> : <span />}
          {tag ?? (example ? <ExampleTag /> : null)}
        </div>
      )}
      {children}
    </section>
  );
}
