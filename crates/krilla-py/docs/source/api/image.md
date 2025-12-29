# Image

Image loading and embedding (requires `raster-images` feature).

## Image

```{eval-rst}
.. autoclass:: krilla.Image
   :members:
   :undoc-members:
   :show-inheritance:
```

Represents a raster image that can be embedded in the PDF. Supports PNG, JPEG, GIF, and WebP formats, as well as PIL/Pillow images.

**Note:** Image support requires the `raster-images` feature to be enabled.

## Utility Functions

```{eval-rst}
.. autofunction:: krilla.has_image_support
```

Check if image support is available in the current build.
