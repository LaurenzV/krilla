"""Tests for graphics state context managers."""

from krilla import (
    Document,
    PageSettings,
    Transform,
    BlendMode,
    PathBuilder,
    Rect,
    Fill,
    Paint,
    NormalizedF32,
    FillRule,
    color,
)


class TestTransformContext:
    """Tests for transform context manager."""

    def test_transform_context_manager(self):
        """Test transform context manager applies and pops correctly."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Initial CTM is identity
                initial_ctm = surface.ctm()
                assert initial_ctm.tx == 0.0
                assert initial_ctm.ty == 0.0

                # Apply transform via context manager
                with surface.transform(Transform.from_translate(50, 100)):
                    ctm = surface.ctm()
                    assert ctm.tx == 50.0
                    assert ctm.ty == 100.0

                # After exit, transform is popped
                final_ctm = surface.ctm()
                assert final_ctm.tx == 0.0
                assert final_ctm.ty == 0.0
        doc.finish()

    def test_nested_transform_contexts(self):
        """Test nesting multiple transform context managers."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                with surface.transform(Transform.from_translate(10, 20)):
                    ctm1 = surface.ctm()
                    assert ctm1.tx == 10.0
                    assert ctm1.ty == 20.0

                    with surface.transform(Transform.from_scale(2.0, 2.0)):
                        ctm2 = surface.ctm()
                        assert ctm2.sx == 2.0  # Scale active
                        assert ctm2.tx == 10.0  # Translation still active

                    # After inner exit, only translation remains
                    ctm3 = surface.ctm()
                    assert ctm3.sx == 1.0  # Scale popped
                    assert ctm3.tx == 10.0  # Translation still there

                # After outer exit, back to identity
                final_ctm = surface.ctm()
                assert final_ctm.tx == 0.0
        doc.finish()


class TestBlendModeContext:
    """Tests for blend mode context manager."""

    def test_blend_mode_context_manager(self):
        """Test blend mode context manager."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Should not crash
                with surface.blend_mode(BlendMode.Multiply):
                    surface.set_fill(Fill(
                        paint=Paint.from_rgb(color.rgb(255, 0, 0)),
                        opacity=NormalizedF32.one(),
                    ))
                    path = PathBuilder()
                    path.push_rect(Rect.from_xywh(10, 10, 40, 40))
                    surface.draw_path(path.finish())
                # Blend mode popped automatically
        doc.finish()

    def test_multiple_blend_modes(self):
        """Test stacking different blend modes."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                with surface.blend_mode(BlendMode.Multiply):
                    with surface.blend_mode(BlendMode.Screen):
                        # Both blend modes active
                        pass
                # All blend modes popped
        doc.finish()


class TestOpacityContext:
    """Tests for opacity context manager."""

    def test_opacity_context_manager(self):
        """Test opacity context manager."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                with surface.opacity(NormalizedF32(0.5)):
                    surface.set_fill(Fill(
                        paint=Paint.from_rgb(color.rgb(0, 255, 0)),
                        opacity=NormalizedF32.one(),
                    ))
                    path = PathBuilder()
                    path.push_rect(Rect.from_xywh(10, 10, 40, 40))
                    surface.draw_path(path.finish())
        doc.finish()


class TestIsolatedContext:
    """Tests for isolated transparency group context manager."""

    def test_isolated_context_manager(self):
        """Test isolated transparency group context manager."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                with surface.isolated():
                    surface.set_fill(Fill(
                        paint=Paint.from_rgb(color.rgb(0, 0, 255)),
                        opacity=NormalizedF32.one(),
                    ))
                    path = PathBuilder()
                    path.push_rect(Rect.from_xywh(10, 10, 40, 40))
                    surface.draw_path(path.finish())
        doc.finish()


class TestClipPathContext:
    """Tests for clip path context manager."""

    def test_clip_path_context_manager(self):
        """Test clip path context manager."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Create a clip path
                clip_path_builder = PathBuilder()
                clip_path_builder.push_rect(Rect.from_xywh(20, 20, 60, 60))
                clip_path = clip_path_builder.finish()

                with surface.clip_path(clip_path, FillRule.NonZero):
                    # Draw something that should be clipped
                    surface.set_fill(Fill(
                        paint=Paint.from_rgb(color.rgb(0, 0, 255)),
                        opacity=NormalizedF32.one(),
                    ))

                    path_builder2 = PathBuilder()
                    path_builder2.push_rect(Rect.from_xywh(0, 0, 100, 100))
                    surface.draw_path(path_builder2.finish())
        doc.finish()


class TestExceptionSafety:
    """Tests for exception safety in context managers."""

    def test_exception_safety(self):
        """Test that pop is called even when exception occurs."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                try:
                    with surface.transform(Transform.from_translate(50, 50)):
                        ctm = surface.ctm()
                        assert ctm.tx == 50.0
                        raise ValueError("Test exception")
                except ValueError:
                    pass

                # Transform should still be popped despite exception
                final_ctm = surface.ctm()
                assert final_ctm.tx == 0.0
        doc.finish()

    def test_exception_in_nested_contexts(self):
        """Test exception handling with nested context managers."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                try:
                    with surface.transform(Transform.from_translate(10, 20)):
                        with surface.blend_mode(BlendMode.Multiply):
                            raise RuntimeError("Test error")
                except RuntimeError:
                    pass

                # All should be popped
                ctm = surface.ctm()
                assert ctm.tx == 0.0
        doc.finish()


class TestComplexNesting:
    """Tests for complex nesting of different context manager types."""

    def test_complex_nesting(self):
        """Test complex nesting of different context manager types."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                with surface.transform(Transform.from_translate(50, 50)):
                    with surface.blend_mode(BlendMode.Multiply):
                        with surface.opacity(NormalizedF32(0.7)):
                            with surface.isolated():
                                # All operations active
                                surface.set_fill(Fill(
                                    paint=Paint.from_rgb(color.rgb(128, 128, 128)),
                                    opacity=NormalizedF32.one(),
                                ))
                                path = PathBuilder()
                                path.push_rect(Rect.from_xywh(0, 0, 30, 30))
                                surface.draw_path(path.finish())
                            # All automatically popped in reverse order

                # Back to clean state
                final_ctm = surface.ctm()
                assert final_ctm.tx == 0.0
        doc.finish()

    def test_interleaved_contexts(self):
        """Test interleaving different types of context managers."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                with surface.transform(Transform.from_translate(10, 10)):
                    with surface.blend_mode(BlendMode.Multiply):
                        with surface.transform(Transform.from_scale(2.0, 2.0)):
                            ctm = surface.ctm()
                            assert ctm.sx == 2.0
                            assert ctm.tx == 10.0
        doc.finish()


class TestMixedStyles:
    """Tests for mixing context manager and manual push/pop styles."""

    def test_context_manager_then_manual(self):
        """Test using context manager followed by manual push/pop."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Context manager style
                with surface.transform(Transform.from_translate(10, 20)):
                    ctm1 = surface.ctm()
                    assert ctm1.tx == 10.0

                # Manual style
                surface.push_transform(Transform.from_translate(30, 40))
                ctm2 = surface.ctm()
                assert ctm2.tx == 30.0
                surface.pop()

                # Back to identity
                final_ctm = surface.ctm()
                assert final_ctm.tx == 0.0
        doc.finish()

    def test_manual_then_context_manager(self):
        """Test using manual push/pop followed by context manager."""
        doc = Document()
        with doc.start_page_with(PageSettings.from_wh(200, 200)) as page:
            with page.surface() as surface:
                # Manual style
                surface.push_transform(Transform.from_translate(10, 20))
                ctm1 = surface.ctm()
                assert ctm1.tx == 10.0

                # Context manager style
                with surface.transform(Transform.from_scale(2.0, 2.0)):
                    ctm2 = surface.ctm()
                    assert ctm2.sx == 2.0
                    assert ctm2.tx == 10.0

                # Original transform still there
                ctm3 = surface.ctm()
                assert ctm3.tx == 10.0
                assert ctm3.sx == 1.0

                surface.pop()

                # Back to identity
                final_ctm = surface.ctm()
                assert final_ctm.tx == 0.0
        doc.finish()
