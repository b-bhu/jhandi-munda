/*
 * The image pack: real art from docs/characters and docs/reference, resized to WebP in
 * public/img (originals stay in docs/). Every image has its size and alt text here, so a page
 * never invents either. Decorative use passes alt="" at the call site.
 */
export interface ImageInfo { src: string; width: number; height: number; alt: string; caption: string }

export const IMAGES = {
  street: { src: "/img/street-scene-wide.webp", width: 1280, height: 720, alt: "A mela street at dusk with the dice table and a crowd standing around it", caption: "A street table at dusk" },
  board: { src: "/img/board.webp", width: 900, height: 675, alt: "The worn cream cloth board: heart, spade, diamond on top; club, crown, flag below", caption: "The six-symbol cloth" },
  dealer: { src: "/img/dealer-sheet.webp", width: 1280, height: 720, alt: "The dealer: a showman with slick hair, a moustache, a red tilak and a paisley shirt, in three poses", caption: "The dealer" },
  diceAndBowl: { src: "/img/dice-and-bowl.webp", width: 1000, height: 750, alt: "Six wooden symbol dice beside a steel bowl and its plate lid", caption: "Six dice, a steel bowl and its lid" },
  bowl: { src: "/img/bowl-crop.webp", width: 504, height: 480, alt: "The steel bowl, covered with its lid", caption: "The covered bowl" },
} as const satisfies Record<string, ImageInfo>;

export type ImageKey = keyof typeof IMAGES;
