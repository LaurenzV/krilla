"""Extension methods for Surface to support context managers."""

from krilla._krilla import Surface
from krilla.context_managers import (
    BlendModeContext,
    ClipPathContext,
    IsolatedContext,
    MaskContext,
    OpacityContext,
    TransformContext,
)


def transform(self, matrix):
    """
    Create a context manager for applying a transform.

    Usage:
        with surface.transform(Transform.from_translate(10, 20)):
            # draw with transform applied
        # transform automatically popped

    Args:
        matrix: Transform to apply

    Returns:
        TransformContext: Context manager that applies transform on enter, pops on exit
    """
    return TransformContext(self, matrix)


def blend_mode(self, mode):
    """
    Create a context manager for applying a blend mode.

    Usage:
        with surface.blend_mode(BlendMode.Multiply):
            # draw with blend mode applied
        # blend mode automatically popped

    Args:
        mode: BlendMode to apply

    Returns:
        BlendModeContext: Context manager that applies blend mode on enter, pops on exit
    """
    return BlendModeContext(self, mode)


def clip_path(self, path, fill_rule):
    """
    Create a context manager for applying a clip path.

    Usage:
        with surface.clip_path(path, FillRule.NonZero):
            # draw with clipping applied
        # clip path automatically popped

    Args:
        path: Path to use for clipping
        fill_rule: FillRule for the clip path

    Returns:
        ClipPathContext: Context manager that applies clip path on enter, pops on exit
    """
    return ClipPathContext(self, path, fill_rule)


def mask(self, mask_obj):
    """
    Create a context manager for applying a mask.

    Usage:
        with surface.mask(mask_obj):
            # draw with mask applied
        # mask automatically popped

    Args:
        mask_obj: Mask to apply

    Returns:
        MaskContext: Context manager that applies mask on enter, pops on exit
    """
    return MaskContext(self, mask_obj)


def opacity(self, opacity_val):
    """
    Create a context manager for applying opacity.

    Usage:
        with surface.opacity(NormalizedF32(0.5)):
            # draw with opacity applied
        # opacity automatically popped

    Args:
        opacity_val: NormalizedF32 opacity value

    Returns:
        OpacityContext: Context manager that applies opacity on enter, pops on exit
    """
    return OpacityContext(self, opacity_val)


def isolated(self):
    """
    Create a context manager for an isolated transparency group.

    Usage:
        with surface.isolated():
            # draw in isolated group
        # isolated group automatically popped

    Returns:
        IsolatedContext: Context manager that pushes isolated group on enter,
            pops on exit
    """
    return IsolatedContext(self)


# Monkey-patch the Surface class
Surface.transform = transform
Surface.blend_mode = blend_mode
Surface.clip_path = clip_path
Surface.mask = mask
Surface.opacity = opacity
Surface.isolated = isolated
