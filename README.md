# krilla

The goal of `krilla` is to be a well-tested and robust high-level 2D graphics library that allows you to build vector graphics with primitives inspired by other graphics libaries, such as `skia` or `cairo`. It's still very much a WIP and not
useable yet, hence the lack of documentation and also code-cleanliness. For now, it's more of a playground for me to
explore some ideas in my mind. The primary export target of `krilla` is PDF and the API will be tailored towards that
to support advanced use cases like tagged PDF and exporting with PDF/A, but depending on how the crate evolves, 
I also want to explore supporting PNG and SVG as second-class export targets.