# Color

Color types and the color module for creating colors.

## Color Module Functions

```{eval-rst}
.. autofunction:: krilla.color.rgb
.. autofunction:: krilla.color.luma
.. autofunction:: krilla.color.cmyk
```

Convenient functions for creating color instances.

## Color Types

### Color

```{eval-rst}
.. autoclass:: krilla.color.Color
   :members:
   :undoc-members:
   :show-inheritance:
```

Generic color wrapper that can hold any color type.

### RgbColor

```{eval-rst}
.. autoclass:: krilla.color.RgbColor
   :members:
   :undoc-members:
   :show-inheritance:
```

RGB color in the sRGB color space with 8-bit channels (0-255).

### LumaColor

```{eval-rst}
.. autoclass:: krilla.color.LumaColor
   :members:
   :undoc-members:
   :show-inheritance:
```

Grayscale color with a single luminance channel (0-255).

### CmykColor

```{eval-rst}
.. autoclass:: krilla.color.CmykColor
   :members:
   :undoc-members:
   :show-inheritance:
```

CMYK color for print output with 8-bit channels (0-255) for cyan, magenta, yellow, and key (black).
