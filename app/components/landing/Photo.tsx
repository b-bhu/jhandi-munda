import Image from "next/image";
import { IMAGES, type ImageKey } from "@/lib/images";

/** A framed real photo from the image pack, with its caption. `decorative` gives it an empty alt. */
export function Photo({ k, decorative = false, caption = true }: { k: ImageKey; decorative?: boolean; caption?: boolean }) {
  const im = IMAGES[k];
  return (
    <figure className="lp-fig">
      <div className="photo"><Image src={im.src} width={im.width} height={im.height} alt={decorative ? "" : im.alt} sizes="(max-width: 760px) calc(100vw - 48px), 540px" /></div>
      {caption && <figcaption className="cap">{im.caption}</figcaption>}
    </figure>
  );
}
