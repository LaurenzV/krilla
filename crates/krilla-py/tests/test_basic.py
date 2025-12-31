"""Basic tests for krilla Python bindings."""

import pytest
from krilla import (
    BlendMode,
    Configuration,
    Document,
    Fill,
    FillRule,
    GlyphId,
    LinearGradient,
    LineCap,
    LineJoin,
    Mask,
    MaskType,
    NormalizedF32,
    PageSettings,
    Paint,
    PathBuilder,
    Pattern,
    PdfVersion,
    Point,
    RadialGradient,
    Rect,
    SerializeSettings,
    Size,
    SpreadMethod,
    Stop,
    StreamBuilder,
    Stroke,
    StrokeDash,
    SweepGradient,
    Transform,
    Validator,
    color,
    has_image_support,
    has_text_support,
)


class TestNormalizedF32:
    def test_valid_values(self):
        assert NormalizedF32(0.0).get() == 0.0
        assert NormalizedF32(0.5).get() == 0.5
        assert NormalizedF32(1.0).get() == 1.0

    def test_invalid_values(self):
        with pytest.raises(ValueError):
            NormalizedF32(-0.1)
        with pytest.raises(ValueError):
            NormalizedF32(1.1)

    def test_static_constructors(self):
        assert NormalizedF32.zero().get() == 0.0
        assert NormalizedF32.one().get() == 1.0

    def test_float_conversion(self):
        n = NormalizedF32(0.75)
        assert float(n) == 0.75

    def test_equality(self):
        assert NormalizedF32(0.5) == NormalizedF32(0.5)
        assert NormalizedF32(0.5) != NormalizedF32(0.6)

    def test_hash(self):
        s = {NormalizedF32(0.5), NormalizedF32(0.5)}
        assert len(s) == 1


class TestGeometry:
    def test_point(self):
        p = Point.from_xy(10.0, 20.0)
        assert p.x == 10.0
        assert p.y == 20.0

    def test_size(self):
        s = Size.from_wh(100.0, 200.0)
        assert s is not None
        assert s.width == 100.0
        assert s.height == 200.0

    def test_size_invalid(self):
        with pytest.raises(ValueError):
            Size.from_wh(-1.0, 100.0)
        with pytest.raises(ValueError):
            Size.from_wh(100.0, 0.0)

    def test_rect_xywh(self):
        r = Rect.from_xywh(10.0, 20.0, 100.0, 200.0)
        assert r is not None
        assert r.left == 10.0
        assert r.top == 20.0
        assert r.width == 100.0
        assert r.height == 200.0

    def test_rect_ltrb(self):
        r = Rect.from_ltrb(10.0, 20.0, 110.0, 220.0)
        assert r is not None
        assert r.left == 10.0
        assert r.top == 20.0
        assert r.right == 110.0
        assert r.bottom == 220.0

    def test_transform_identity(self):
        t = Transform.identity()
        assert t.sx == 1.0
        assert t.sy == 1.0
        assert t.tx == 0.0
        assert t.ty == 0.0

    def test_transform_translate(self):
        t = Transform.from_translate(10.0, 20.0)
        assert t.tx == 10.0
        assert t.ty == 20.0

    def test_transform_scale(self):
        t = Transform.from_scale(2.0, 3.0)
        assert t.sx == 2.0
        assert t.sy == 3.0

    def test_transform_invert(self):
        t = Transform.from_translate(10.0, 20.0)
        inv = t.invert()
        assert inv is not None
        assert inv.tx == -10.0
        assert inv.ty == -20.0

    def test_transform_matmul(self):
        # Test transform composition with @ operator
        # Translate then scale
        translate = Transform.from_translate(10.0, 20.0)
        scale = Transform.from_scale(2.0, 3.0)

        # combined = translate @ scale means: apply scale first, then translate
        combined = translate @ scale
        assert combined.sx == 2.0
        assert combined.sy == 3.0
        assert combined.tx == 10.0
        assert combined.ty == 20.0

        # Verify it's different from the reverse order
        combined_reverse = scale @ translate
        assert combined_reverse.sx == 2.0
        assert combined_reverse.sy == 3.0
        # After scaling, the translation is also scaled
        assert combined_reverse.tx == 20.0  # 10.0 * 2.0
        assert combined_reverse.ty == 60.0  # 20.0 * 3.0

    def test_transform_matmul_identity(self):
        # Identity is neutral element
        t = Transform.from_translate(5.0, 10.0)
        identity = Transform.identity()

        result1 = t @ identity
        assert result1.tx == 5.0
        assert result1.ty == 10.0

        result2 = identity @ t
        assert result2.tx == 5.0
        assert result2.ty == 10.0

    def test_transform_matmul_multiple(self):
        # Chain multiple transforms
        t1 = Transform.from_translate(10.0, 0.0)
        t2 = Transform.from_scale(2.0, 2.0)
        t3 = Transform.from_rotate(90.0)

        # Apply t3, then t2, then t1
        combined = t1 @ t2 @ t3
        assert combined is not None


class TestPath:
    def test_path_builder_triangle(self):
        pb = PathBuilder()
        pb.move_to(100.0, 20.0)
        pb.line_to(160.0, 160.0)
        pb.line_to(40.0, 160.0)
        pb.close()
        path = pb.finish()
        assert path is not None

    def test_path_builder_rect(self):
        pb = PathBuilder()
        rect = Rect.from_xywh(10.0, 10.0, 100.0, 100.0)
        pb.push_rect(rect)
        path = pb.finish()
        assert path is not None

    def test_path_builder_curves(self):
        pb = PathBuilder()
        pb.move_to(0.0, 0.0)
        pb.quad_to(50.0, 100.0, 100.0, 0.0)
        pb.cubic_to(150.0, 100.0, 200.0, 100.0, 250.0, 0.0)
        path = pb.finish()
        assert path is not None

    def test_path_builder_consumed(self):
        pb = PathBuilder()
        pb.move_to(0.0, 0.0)
        pb.line_to(100.0, 100.0)
        pb.finish()
        with pytest.raises(RuntimeError):
            pb.finish()

    def test_path_transform(self):
        pb = PathBuilder()
        pb.move_to(0.0, 0.0)
        pb.line_to(100.0, 100.0)
        path = pb.finish()
        assert path is not None

        t = Transform.from_scale(2.0, 2.0)
        transformed = path.transform(t)
        assert transformed is not None


class TestColors:
    def test_rgb(self):
        c = color.rgb(255, 128, 0)
        assert c.red == 255
        assert c.green == 128
        assert c.blue == 0

    def test_rgb_class(self):
        c = color.RgbColor(100, 150, 200)
        assert c.red == 100
        assert c.green == 150
        assert c.blue == 200

    def test_rgb_black_white(self):
        black = color.RgbColor.black()
        white = color.RgbColor.white()
        assert black.red == 0 and black.green == 0 and black.blue == 0
        assert white.red == 255 and white.green == 255 and white.blue == 255

    def test_luma(self):
        c = color.luma(128)
        assert c.lightness == 128

    def test_cmyk(self):
        c = color.cmyk(100, 50, 25, 10)
        assert c.cyan == 100
        assert c.magenta == 50
        assert c.yellow == 25
        assert c.black == 10

    def test_color_from_rgb(self):
        rgb = color.rgb(255, 0, 0)
        c = color.Color.from_rgb(rgb)
        assert c is not None


class TestPaint:
    def test_fill_simple(self):
        fill = Fill(paint=Paint.from_rgb(color.rgb(255, 0, 0)))
        assert fill.opacity.get() == 1.0
        assert fill.rule == FillRule.NonZero

    def test_fill_with_options(self):
        fill = Fill(
            paint=Paint.from_rgb(color.rgb(0, 255, 0)),
            opacity=NormalizedF32(0.5),
            rule=FillRule.EvenOdd,
        )
        assert fill.opacity.get() == 0.5
        assert fill.rule == FillRule.EvenOdd

    def test_stroke_simple(self):
        stroke = Stroke(paint=Paint.from_rgb(color.rgb(0, 0, 255)), width=2.0)
        assert stroke.width == 2.0
        assert stroke.line_cap == LineCap.Butt
        assert stroke.line_join == LineJoin.Miter

    def test_stroke_with_options(self):
        stroke = Stroke(
            paint=Paint.from_rgb(color.rgb(0, 0, 0)),
            width=3.0,
            opacity=NormalizedF32(0.8),
            line_cap=LineCap.Round,
            line_join=LineJoin.Round,
            miter_limit=2.0,
        )
        assert stroke.width == 3.0
        assert stroke.opacity.get() == pytest.approx(0.8)
        assert stroke.line_cap == LineCap.Round
        assert stroke.line_join == LineJoin.Round

    def test_stroke_dash(self):
        dash = StrokeDash([5.0, 3.0], 0.0)
        stroke = Stroke(paint=Paint.from_rgb(color.rgb(0, 0, 0)), width=1.0, dash=dash)
        assert stroke is not None

    def test_stop(self):
        c = color.Color.from_rgb(color.rgb(255, 0, 0))
        stop = Stop(NormalizedF32(0.5), c)
        assert stop.offset.get() == 0.5

    def test_linear_gradient(self):
        stops = [
            Stop(NormalizedF32(0.0), color.Color.from_rgb(color.rgb(255, 0, 0))),
            Stop(NormalizedF32(1.0), color.Color.from_rgb(color.rgb(0, 0, 255))),
        ]
        grad = LinearGradient(0.0, 0.0, 100.0, 100.0, stops)
        assert grad is not None

    def test_radial_gradient(self):
        stops = [
            Stop(NormalizedF32(0.0), color.Color.from_rgb(color.rgb(255, 255, 0))),
            Stop(NormalizedF32(1.0), color.Color.from_rgb(color.rgb(0, 255, 255))),
        ]
        grad = RadialGradient(50.0, 50.0, 50.0, 50.0, 50.0, 0.0, stops)
        assert grad is not None

    def test_sweep_gradient(self):
        stops = [
            Stop(NormalizedF32(0.0), color.Color.from_rgb(color.rgb(255, 0, 255))),
            Stop(NormalizedF32(1.0), color.Color.from_rgb(color.rgb(0, 255, 0))),
        ]
        grad = SweepGradient(50.0, 50.0, 0.0, 360.0, stops)
        assert grad is not None


class TestEnums:
    def test_fill_rule(self):
        assert FillRule.NonZero != FillRule.EvenOdd

    def test_line_cap(self):
        assert LineCap.Butt != LineCap.Round
        assert LineCap.Round != LineCap.Square

    def test_line_join(self):
        assert LineJoin.Miter != LineJoin.Round
        assert LineJoin.Round != LineJoin.Bevel

    def test_spread_method(self):
        assert SpreadMethod.Pad != SpreadMethod.Reflect
        assert SpreadMethod.Reflect != SpreadMethod.Repeat

    def test_blend_mode(self):
        assert BlendMode.Normal != BlendMode.Multiply
        assert BlendMode.Screen != BlendMode.Overlay

    def test_mask_type(self):
        assert MaskType.Luminosity != MaskType.Alpha


class TestText:
    def test_glyph_id(self):
        g = GlyphId(42)
        assert g.to_u32() == 42

    def test_glyph_id_equality(self):
        assert GlyphId(10) == GlyphId(10)
        assert GlyphId(10) != GlyphId(20)


class TestConfiguration:
    def test_pdf_version(self):
        # as_str() returns full format like "PDF 1.4"
        assert "1.4" in PdfVersion.Pdf14.as_str()
        assert "2.0" in PdfVersion.Pdf20.as_str()

    def test_validator_compatibility(self):
        v = Validator.A2B
        assert v.compatible_with_version(PdfVersion.Pdf17)

    def test_validator_recommended_version(self):
        v = Validator.A2B
        rec = v.recommended_version()
        assert rec is not None

    def test_configuration_default(self):
        c = Configuration()
        # Use getattr since None is a Python keyword
        assert c.validator == getattr(Validator, "None")
        assert c.version == PdfVersion.Pdf17

    def test_configuration_with_validator(self):
        c = Configuration(validator=Validator.A2B)
        assert c.validator == Validator.A2B

    def test_configuration_with_version(self):
        c = Configuration(version=PdfVersion.Pdf20)
        assert c.version == PdfVersion.Pdf20

    def test_configuration_with_both(self):
        c = Configuration(validator=Validator.A2B, version=PdfVersion.Pdf17)
        assert c.validator == Validator.A2B
        assert c.version == PdfVersion.Pdf17

    def test_configuration_incompatible_raises(self):
        import pytest

        # PDF/A-4 requires PDF 2.0, so PDF 1.7 is incompatible
        with pytest.raises(ValueError):
            Configuration(validator=Validator.A4, version=PdfVersion.Pdf17)

    def test_serialize_settings(self):
        s = SerializeSettings()
        assert s is not None

    def test_serialize_settings_with_config(self):
        c = Configuration(validator=Validator.A2B)
        s = SerializeSettings(configuration=c)
        assert s is not None

    def test_serialize_settings_with_options(self):
        s = SerializeSettings(compress=False, xmp_metadata=True, enable_tagging=True)
        assert s is not None
        assert "compress=false" in repr(s).lower()
        assert "xmp_metadata=true" in repr(s).lower()
        assert "enable_tagging=true" in repr(s).lower()


class TestPageSettings:
    def test_page_settings_from_size(self):
        size = Size.from_wh(612.0, 792.0)  # US Letter
        ps = PageSettings(size)
        assert ps is not None

    def test_page_settings_from_wh(self):
        ps = PageSettings.from_wh(595.0, 842.0)  # A4
        assert ps is not None

    def test_page_settings_with_boxes(self):
        ps = PageSettings.from_wh(200.0, 200.0)
        crop = Rect.from_xywh(10.0, 10.0, 180.0, 180.0)
        bleed = Rect.from_xywh(5.0, 5.0, 190.0, 190.0)
        ps2 = ps.with_page_boxes(crop_box=crop, bleed_box=bleed)
        assert ps2 is not None


class TestFeatureDetection:
    def test_has_image_support(self):
        # Should return True since raster-images is a default feature
        result = has_image_support()
        assert isinstance(result, bool)

    def test_has_text_support(self):
        # Should return True since simple-text is a default feature
        result = has_text_support()
        assert isinstance(result, bool)


class TestDocument:
    def test_create_document(self):
        doc = Document()
        assert doc is not None

    def test_create_document_with_settings(self):
        settings = SerializeSettings()
        doc = Document.new_with(settings)
        assert doc is not None

    def test_empty_document(self):
        doc = Document()
        pdf_bytes = doc.finish()
        assert isinstance(pdf_bytes, bytes)
        assert len(pdf_bytes) > 0
        assert pdf_bytes.startswith(b"%PDF")

    def test_single_page_document(self):
        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page, page.surface() as surface:
            # Draw a simple rectangle
            pb = PathBuilder()
            pb.push_rect(Rect.from_xywh(10.0, 10.0, 50.0, 50.0))
            path = pb.finish()

            surface.set_fill(Fill(paint=Paint.from_rgb(color.rgb(255, 0, 0))))
            surface.draw_path(path)

        pdf_bytes = doc.finish()
        assert isinstance(pdf_bytes, bytes)
        assert len(pdf_bytes) > 0

    def test_surface_transforms(self):
        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page, page.surface() as surface:
            # Initial transform should be identity
            initial_ctm = surface.ctm()
            assert initial_ctm.sx == 1.0
            assert initial_ctm.ky == 0.0
            assert initial_ctm.tx == 0.0  # No translation
            assert initial_ctm.ty == 0.0

            # Test push/pop transform
            surface.push_transform(Transform.from_translate(50.0, 100.0))

            # CTM should now include the translation
            ctm = surface.ctm()
            assert ctm.sx == 1.0  # Scale unchanged
            assert ctm.ky == 0.0  # Rotation unchanged
            assert ctm.tx == 50.0  # X translation
            assert ctm.ty == 100.0  # Y translation

            # Push another transform (concatenates)
            surface.push_transform(Transform.from_scale(2.0, 2.0))
            ctm2 = surface.ctm()
            assert ctm2.sx == 2.0  # Scale applied
            assert ctm2.sy == 2.0  # Scale applied

            # Pop restores previous state
            surface.pop()
            ctm3 = surface.ctm()
            assert ctm3.tx == 50.0  # Back to translation only
            assert ctm3.sx == 1.0  # Scale removed

            surface.pop()

        doc.finish()

    def test_surface_blend_mode(self):
        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page, page.surface() as surface:
            surface.push_blend_mode(BlendMode.Multiply)
            surface.pop()

        doc.finish()

    def test_surface_opacity(self):
        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page, page.surface() as surface:
            surface.push_opacity(NormalizedF32(0.5))
            surface.pop()

        doc.finish()

    def test_multi_page_document(self):
        doc = Document()

        for i in range(3):
            ps = PageSettings.from_wh(200.0, 200.0)
            with doc.start_page_with(ps) as page, page.surface() as surface:
                pb = PathBuilder()
                pb.push_rect(Rect.from_xywh(10.0, 10.0, 50.0, 50.0))
                path = pb.finish()
                surface.set_fill(
                    Fill(paint=Paint.from_rgb(color.rgb(i * 80, 100, 100)))
                )
                surface.draw_path(path)

        pdf_bytes = doc.finish()
        assert len(pdf_bytes) > 0


class TestStreamAndMask:
    def test_stream_builder(self):
        size = Size.from_wh(100.0, 100.0)
        builder = StreamBuilder(size)

        surface = builder.surface()
        pb = PathBuilder()
        pb.push_rect(Rect.from_xywh(0.0, 0.0, 100.0, 100.0))
        path = pb.finish()
        surface.set_fill(Fill(paint=Paint.from_rgb(color.rgb(255, 255, 255))))
        surface.draw_path(path)
        surface.finish()

        stream = builder.finish()
        assert stream is not None

    def test_stream_builder_with_context_manager(self):
        size = Size.from_wh(100.0, 100.0)
        builder = StreamBuilder(size)

        with builder.surface() as surface:
            pb = PathBuilder()
            pb.push_rect(Rect.from_xywh(0.0, 0.0, 100.0, 100.0))
            path = pb.finish()
            surface.set_fill(Fill(paint=Paint.from_rgb(color.rgb(255, 0, 0))))
            surface.draw_path(path)

        stream = builder.finish()
        assert stream is not None

    def test_mask(self):
        size = Size.from_wh(100.0, 100.0)
        builder = StreamBuilder(size)

        with builder.surface() as surface:
            pb = PathBuilder()
            pb.push_rect(Rect.from_xywh(0.0, 0.0, 100.0, 100.0))
            path = pb.finish()
            surface.set_fill(Fill(paint=Paint.from_luma(color.luma(255))))
            surface.draw_path(path)

        stream = builder.finish()
        mask = Mask(stream, MaskType.Luminosity)
        assert mask is not None

    def test_pattern(self):
        size = Size.from_wh(20.0, 20.0)
        builder = StreamBuilder(size)

        with builder.surface() as surface:
            pb = PathBuilder()
            pb.push_rect(Rect.from_xywh(0.0, 0.0, 10.0, 10.0))
            path = pb.finish()
            surface.set_fill(Fill(paint=Paint.from_rgb(color.rgb(255, 0, 0))))
            surface.draw_path(path)

        stream = builder.finish()
        pattern = Pattern(stream, width=20.0, height=20.0)
        assert pattern is not None
        assert pattern.width == 20.0
        assert pattern.height == 20.0

    def test_stream_builder_push_pop(self):
        size = Size.from_wh(100.0, 100.0)
        builder = StreamBuilder(size)

        with builder.surface() as surface:
            surface.push_opacity(NormalizedF32(0.5))
            pb = PathBuilder()
            pb.push_rect(Rect.from_xywh(0.0, 0.0, 50.0, 50.0))
            path = pb.finish()
            surface.set_fill(Fill(paint=Paint.from_rgb(color.rgb(255, 0, 0))))
            surface.draw_path(path)
            surface.pop()

        stream = builder.finish()
        assert stream is not None

    def test_stream_builder_transform(self):
        size = Size.from_wh(100.0, 100.0)
        builder = StreamBuilder(size)

        with builder.surface() as surface:
            surface.push_transform(Transform.from_translate(10.0, 10.0))
            pb = PathBuilder()
            pb.push_rect(Rect.from_xywh(0.0, 0.0, 50.0, 50.0))
            path = pb.finish()
            surface.set_fill(Fill(paint=Paint.from_rgb(color.rgb(0, 255, 0))))
            surface.draw_path(path)
            surface.pop()

        stream = builder.finish()
        assert stream is not None


class TestAccessibility:
    """Tests for accessibility/tagging features."""

    def test_location(self):
        from krilla import Location

        loc = Location(42)
        assert loc.get() == 42
        assert repr(loc) == "Location(42)"

        loc2 = Location(42)
        assert loc == loc2

    def test_location_zero_raises(self):
        from krilla import Location

        with pytest.raises(ValueError, match="non-zero"):
            Location(0)

    def test_artifact_type(self):
        from krilla import ArtifactType

        assert ArtifactType.Header is not None
        assert ArtifactType.Footer is not None
        assert ArtifactType.Page is not None
        assert ArtifactType.Other is not None

    def test_span_tag(self):
        from krilla import SpanTag

        # Default (all None)
        tag = SpanTag()
        assert tag.lang is None
        assert tag.alt_text is None
        assert tag.expanded is None
        assert tag.actual_text is None

        # With values
        tag = SpanTag(
            lang="en-US", alt_text="Description", expanded="abbrev", actual_text="text"
        )
        assert tag.lang == "en-US"
        assert tag.alt_text == "Description"
        assert tag.expanded == "abbrev"
        assert tag.actual_text == "text"

        # Mutable properties
        tag.lang = "de-DE"
        assert tag.lang == "de-DE"

    def test_content_tag_artifact(self):
        from krilla import ArtifactType, ContentTag

        tag = ContentTag.artifact(ArtifactType.Header)
        assert "Artifact" in repr(tag)

    def test_content_tag_span(self):
        from krilla import ContentTag, SpanTag

        span = SpanTag(lang="en-US")
        tag = ContentTag.span(span)
        assert "Span" in repr(tag)

    def test_content_tag_other(self):
        from krilla import ContentTag

        tag = ContentTag.other()
        assert "Other" in repr(tag)

    def test_identifier(self):
        from krilla import ArtifactType, ContentTag, Document, PageSettings

        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page, page.surface() as surface:
            tag = ContentTag.artifact(ArtifactType.Other)
            identifier = surface.start_tagged(tag)
            assert identifier is not None
            assert identifier.is_dummy()  # Artifacts return dummy identifiers
            surface.end_tagged()

        doc.finish()

    def test_surface_location(self):
        from krilla import Document, Location, PageSettings

        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page, page.surface() as surface:
            # Initially no location
            assert surface.get_location() is None

            # Set location
            loc = Location(123)
            surface.set_location(loc)
            retrieved = surface.get_location()
            assert retrieved is not None
            assert retrieved.get() == 123

            # Reset location
            surface.reset_location()
            assert surface.get_location() is None

        doc.finish()

    def test_surface_tagged_balanced(self):
        from krilla import ContentTag, Document, PageSettings, SpanTag

        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page, page.surface() as surface:
            # Balanced tagged sections
            span = SpanTag(lang="en-US")
            tag = ContentTag.span(span)
            identifier = surface.start_tagged(tag)
            assert identifier is not None
            surface.end_tagged()

            # Multiple nested
            surface.start_tagged(ContentTag.other())
            surface.start_tagged(ContentTag.other())
            surface.end_tagged()
            surface.end_tagged()

        doc.finish()

    def test_surface_tagged_unbalanced_raises(self):
        from krilla import ContentTag, Document, PageSettings

        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page:
            surface = page.surface()
            surface.start_tagged(ContentTag.other())
            # Missing end_tagged() - should raise on finish
            with pytest.raises(RuntimeError, match="unbalanced tagged"):
                surface.finish()

    def test_surface_end_tagged_without_start_raises(self):
        from krilla import Document, PageSettings

        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page:
            surface = page.surface()
            with pytest.raises(RuntimeError, match="without matching start_tagged"):
                surface.end_tagged()
            surface.finish()

    def test_surface_alt_text(self):
        from krilla import Document, PageSettings

        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page, page.surface() as surface:
            surface.start_alt_text("Alternative text description")
            surface.end_alt_text()

        doc.finish()

    def test_surface_alt_text_unbalanced_raises(self):
        from krilla import Document, PageSettings

        doc = Document()
        ps = PageSettings.from_wh(200.0, 200.0)

        with doc.start_page_with(ps) as page:
            surface = page.surface()
            surface.start_alt_text("Description")
            # Missing end_alt_text() - should raise on finish
            with pytest.raises(RuntimeError, match="unbalanced tagged"):
                surface.finish()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
