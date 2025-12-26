"""Tests for graphics state management (transforms, blend modes, etc.)."""

import pytest
from krilla import (
    BlendMode,
    Document,
    Fill,
    NormalizedF32,
    PageSettings,
    Paint,
    Path,
    PathBuilder,
    Point,
    Rect,
    Transform,
    color,
)


class TestCTM:
    """Tests for current transformation matrix (CTM)."""

    def test_ctm_identity(self):
        """Test that initial CTM is identity."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                ctm = surface.ctm()
                assert ctm.sx == 1.0 and ctm.sy == 1.0
                assert ctm.ky == 0.0 and ctm.kx == 0.0
                assert ctm.tx == 0.0 and ctm.ty == 0.0
        doc.finish()

    def test_ctm_after_transform(self):
        """Test that CTM reflects applied transforms."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Apply translation
                surface.push_transform(Transform.from_translate(10.0, 20.0))
                ctm = surface.ctm()

                # Verify translation is in CTM
                assert ctm.tx == 10.0
                assert ctm.ty == 20.0

                surface.pop()
        doc.finish()


class TestTransformAccumulation:
    """Tests for transform accumulation and concatenation."""

    def test_transform_accumulation(self):
        """Test that transforms accumulate correctly."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Push translation
                surface.push_transform(Transform.from_translate(10.0, 20.0))
                ctm = surface.ctm()
                assert ctm.tx == 10.0
                assert ctm.ty == 20.0

                # Push another translation - should accumulate
                surface.push_transform(Transform.from_translate(5.0, 10.0))
                ctm = surface.ctm()
                assert ctm.tx == 15.0  # 10 + 5
                assert ctm.ty == 30.0  # 20 + 10

                surface.pop()
                surface.pop()
        doc.finish()

    def test_transform_concatenation(self):
        """Test that transforms concatenate (not just add)."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Translate then scale
                surface.push_transform(Transform.from_translate(100.0, 100.0))
                surface.push_transform(Transform.from_scale(2.0, 2.0))

                ctm = surface.ctm()
                # After scale, translation is also scaled
                assert ctm.sx == 2.0  # Scale
                assert ctm.sy == 2.0  # Scale
                assert ctm.tx == 100.0  # Translation (pre-concat)
                assert ctm.ty == 100.0

                surface.pop()
                surface.pop()
        doc.finish()

    def test_nested_transforms(self):
        """Test deeply nested transform stack."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                transforms = [
                    Transform.from_translate(10, 10),
                    Transform.from_rotate(0.5),
                    Transform.from_scale(1.5, 1.5),
                    Transform.from_translate(20, 20),
                ]

                # Push all transforms
                for t in transforms:
                    surface.push_transform(t)

                # CTM should be concatenation of all
                ctm = surface.ctm()
                # Just verify it's not identity
                assert not (
                    ctm.sx == 1.0
                    and ctm.sy == 1.0
                    and ctm.ky == 0.0
                    and ctm.kx == 0.0
                    and ctm.tx == 0.0
                    and ctm.ty == 0.0
                )

                # Pop all
                for _ in transforms:
                    surface.pop()

                # Back to identity
                final_ctm = surface.ctm()
                assert final_ctm.sx == 1.0
                assert final_ctm.tx == 0.0
        doc.finish()


class TestPopRestoration:
    """Tests for state restoration via pop()."""

    def test_pop_restores_transform(self):
        """Test that pop() restores previous transform."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Push first transform
                surface.push_transform(Transform.from_translate(50.0, 50.0))
                ctm1 = surface.ctm()
                assert ctm1.tx == 50.0

                # Push second transform
                surface.push_transform(Transform.from_scale(2.0, 2.0))
                ctm2 = surface.ctm()
                assert ctm2.sx == 2.0

                # Pop should restore to first transform
                surface.pop()
                ctm3 = surface.ctm()
                assert ctm3.tx == 50.0  # Translation still there
                assert ctm3.sx == 1.0  # Scale removed

                # Pop again should restore to identity
                surface.pop()
                ctm4 = surface.ctm()
                assert ctm4.tx == 0.0
                assert ctm4.sx == 1.0
        doc.finish()

    def test_pop_error_on_underflow(self):
        """Test that pop() errors when stack is empty."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Should error - no matching push
                with pytest.raises(RuntimeError, match="pop\\(\\) called without matching push"):
                    surface.pop()
        doc.finish()


class TestBlendMode:
    """Tests for blend mode application."""

    def test_blend_mode_application(self):
        """Test that blend modes are actually applied."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # This should not crash and should affect rendering
                surface.push_blend_mode(BlendMode.Multiply)

                # Draw something with the blend mode
                surface.set_fill(
                    Fill(
                        paint=Paint.from_rgb(color.rgb(255, 0, 0)),
                        opacity=NormalizedF32.one(),
                    )
                )

                path = PathBuilder()
                path.push_rect(Rect.from_xywh(10, 10, 40, 40))
                surface.draw_path(path.finish())

                surface.pop()
        doc.finish()

    def test_multiple_blend_modes(self):
        """Test stacking different blend modes."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                surface.push_blend_mode(BlendMode.Multiply)
                surface.push_blend_mode(BlendMode.Screen)
                surface.push_blend_mode(BlendMode.Overlay)

                # Just verify no crashes
                surface.pop()
                surface.pop()
                surface.pop()
        doc.finish()


class TestClipPath:
    """Tests for clip path application."""

    def test_clip_path_application(self):
        """Test that clip paths are applied."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Create a clip path
                from krilla import FillRule

                path_builder = PathBuilder()
                path_builder.push_rect(Rect.from_xywh(20, 20, 60, 60))
                clip_path = path_builder.finish()

                surface.push_clip_path(clip_path, FillRule.NonZero)

                # Draw something that should be clipped
                surface.set_fill(
                    Fill(
                        paint=Paint.from_rgb(color.rgb(0, 0, 255)),
                        opacity=NormalizedF32.one(),
                    )
                )

                path_builder2 = PathBuilder()
                path_builder2.push_rect(Rect.from_xywh(0, 0, 100, 100))
                surface.draw_path(path_builder2.finish())

                surface.pop()
        doc.finish()


class TestOpacity:
    """Tests for opacity groups."""

    def test_opacity_application(self):
        """Test that opacity is applied."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Push opacity
                surface.push_opacity(NormalizedF32(0.5))

                # Draw something with opacity
                surface.set_fill(
                    Fill(
                        paint=Paint.from_rgb(color.rgb(255, 0, 0)),
                        opacity=NormalizedF32.one(),
                    )
                )

                path = PathBuilder()
                path.push_rect(Rect.from_xywh(10, 10, 40, 40))
                surface.draw_path(path.finish())

                surface.pop()
        doc.finish()


class TestIsolatedGroup:
    """Tests for isolated transparency groups."""

    def test_isolated_group(self):
        """Test that isolated groups work."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                surface.push_isolated()

                # Draw something in isolated group
                surface.set_fill(
                    Fill(
                        paint=Paint.from_rgb(color.rgb(0, 255, 0)),
                        opacity=NormalizedF32.one(),
                    )
                )

                path = PathBuilder()
                path.push_rect(Rect.from_xywh(10, 10, 40, 40))
                surface.draw_path(path.finish())

                surface.pop()
        doc.finish()


class TestComplexStateStack:
    """Tests for complex combinations of graphics states."""

    def test_mixed_state_operations(self):
        """Test mixing transforms, blend modes, and opacity."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Complex state stack - verify that mixing different push operations works
                surface.push_transform(Transform.from_translate(50, 50))
                surface.push_blend_mode(BlendMode.Multiply)
                surface.push_transform(Transform.from_scale(1.5, 1.5))
                surface.push_opacity(NormalizedF32(0.7))

                # Draw something - this should not crash
                surface.set_fill(
                    Fill(
                        paint=Paint.from_rgb(color.rgb(128, 128, 128)),
                        opacity=NormalizedF32.one(),
                    )
                )
                path = PathBuilder()
                path.push_rect(Rect.from_xywh(0, 0, 30, 30))
                surface.draw_path(path.finish())

                # Pop all - should restore to clean state
                surface.pop()  # opacity
                surface.pop()  # scale
                surface.pop()  # blend mode
                surface.pop()  # translate

                # After popping all, back to identity
                final_ctm = surface.ctm()
                assert final_ctm.sx == 1.0
                assert final_ctm.tx == 0.0
        doc.finish()
