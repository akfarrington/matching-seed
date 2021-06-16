#matching-seed

a basic flashcard matching game made using [seed](https://github.com/seed-rs/seed).

Uses the [image crate](https://crates.io/crates/image) to scale images then help convert them to base64 blobs, but, unfortunately, the image crate's jpg decoder built with target `wasm-unknown-unknown` doesn't work with jpeg files, apparently.