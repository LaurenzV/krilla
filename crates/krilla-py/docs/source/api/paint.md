# Paint & Strokes

Paint types for filling and stroking paths.

## Paint

```{eval-rst}
.. autoclass:: krilla.Paint
   :members:
   :undoc-members:
   :show-inheritance:
```

A paint can be a solid color, gradient, or pattern. Use the `from_*` static methods or gradient `into_paint()` to create a Paint.

## Fill

```{eval-rst}
.. autoclass:: krilla.Fill
   :members:
   :undoc-members:
   :show-inheritance:
```

Defines how to fill the interior of a path.

## Stroke

```{eval-rst}
.. autoclass:: krilla.Stroke
   :members:
   :undoc-members:
   :show-inheritance:
```

Defines how to stroke the outline of a path with configurable width, line caps, joins, and dash patterns.

## Gradients

### LinearGradient

```{eval-rst}
.. autoclass:: krilla.LinearGradient
   :members:
   :undoc-members:
   :show-inheritance:
```

### RadialGradient

```{eval-rst}
.. autoclass:: krilla.RadialGradient
   :members:
   :undoc-members:
   :show-inheritance:
```

### SweepGradient

```{eval-rst}
.. autoclass:: krilla.SweepGradient
   :members:
   :undoc-members:
   :show-inheritance:
```

## Gradient Components

### Stop

```{eval-rst}
.. autoclass:: krilla.Stop
   :members:
   :undoc-members:
   :show-inheritance:
```

Defines a color stop in a gradient at a specific offset position.

### StrokeDash

```{eval-rst}
.. autoclass:: krilla.StrokeDash
   :members:
   :undoc-members:
   :show-inheritance:
```

Defines a dash pattern for stroked paths.
