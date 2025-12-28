"""Context managers for graphics state operations."""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from krilla import (
        BlendMode,
        FillRule,
        Mask,
        NormalizedF32,
        Path,
        Surface,
        Transform,
    )


class TransformContext:
    """Context manager for applying transforms with automatic pop."""

    def __init__(self, surface: 'Surface', transform: 'Transform'):
        self.surface = surface
        self.transform = transform
        self._entered = False

    def __enter__(self) -> 'TransformContext':
        self.surface.push_transform(self.transform)
        self._entered = True
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> bool:
        if self._entered:
            self.surface.pop()
            self._entered = False
        return False  # Don't suppress exceptions


class BlendModeContext:
    """Context manager for applying blend modes with automatic pop."""

    def __init__(self, surface: 'Surface', blend_mode: 'BlendMode'):
        self.surface = surface
        self.blend_mode = blend_mode
        self._entered = False

    def __enter__(self) -> 'BlendModeContext':
        self.surface.push_blend_mode(self.blend_mode)
        self._entered = True
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> bool:
        if self._entered:
            self.surface.pop()
            self._entered = False
        return False


class ClipPathContext:
    """Context manager for applying clip paths with automatic pop."""

    def __init__(self, surface: 'Surface', path: 'Path', fill_rule: 'FillRule'):
        self.surface = surface
        self.path = path
        self.fill_rule = fill_rule
        self._entered = False

    def __enter__(self) -> 'ClipPathContext':
        self.surface.push_clip_path(self.path, self.fill_rule)
        self._entered = True
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> bool:
        if self._entered:
            self.surface.pop()
            self._entered = False
        return False


class MaskContext:
    """Context manager for applying masks with automatic pop."""

    def __init__(self, surface: 'Surface', mask: 'Mask'):
        self.surface = surface
        self.mask = mask
        self._entered = False

    def __enter__(self) -> 'MaskContext':
        self.surface.push_mask(self.mask)
        self._entered = True
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> bool:
        if self._entered:
            self.surface.pop()
            self._entered = False
        return False


class OpacityContext:
    """Context manager for applying opacity with automatic pop."""

    def __init__(self, surface: 'Surface', opacity: 'NormalizedF32'):
        self.surface = surface
        self.opacity = opacity
        self._entered = False

    def __enter__(self) -> 'OpacityContext':
        self.surface.push_opacity(self.opacity)
        self._entered = True
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> bool:
        if self._entered:
            self.surface.pop()
            self._entered = False
        return False


class IsolatedContext:
    """Context manager for isolated transparency groups with automatic pop."""

    def __init__(self, surface: 'Surface'):
        self.surface = surface
        self._entered = False

    def __enter__(self) -> 'IsolatedContext':
        self.surface.push_isolated()
        self._entered = True
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> bool:
        if self._entered:
            self.surface.pop()
            self._entered = False
        return False
