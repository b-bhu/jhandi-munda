Resized WebP copies of the art in `docs/characters/` (board, dealer-sheet, dice-and-bowl),
`docs/reference/` (street-scene-wide) and `docs/design/app-html/img/` (bowl-crop).
Originals stay in `docs/`. Sizes and alt text: `app/lib/images.ts`.
Regenerate with Pillow: resize to the widths in that file, `save(..., "WEBP", quality=80)`.
