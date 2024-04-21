# senteru
A website for booking hotels next to major train lines.

## Name
sen = line, teru (hoteru) = hotel

## How to run
1. Clone the repository
2. Download an OSM dump of your desired area; e.g. https://download.bbbike.org/osm/bbbike/Tokyo/
3. run `cargo run -- --import` if it's the first time
4. afterwards, just run `cargo run`
5. start the vite server with `cd web && pnpm run dev` (or `npm run dev` if you don't have pnpm installed)