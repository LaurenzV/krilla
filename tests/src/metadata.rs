use krilla::configure::ValidationError;
use krilla::embed::{AssociationKind, EmbeddedFile, MimeType};
use krilla::error::KrillaError;
use krilla::metadata::xmp::{Category, Namespace, Property, StructField, Value, XmpError};
use krilla::metadata::{DateTime, Metadata, PageLayout, TextDirection};
use krilla::Document;
use krilla_macros::snapshot;

use crate::{settings_10, settings_19};

fn datetime() -> DateTime {
    DateTime::new(2024)
        .month(11)
        .day(8)
        .hour(22)
        .minute(23)
        .second(18)
        .utc_offset_hour(1)
        .utc_offset_minute(12)
}

pub(crate) fn metadata_impl(document: &mut Document) {
    let date = datetime();
    let metadata = Metadata::new()
        .creation_date(date)
        .description("A very interesting subject".to_string())
        .creator("krilla".to_string())
        .producer("krilla".to_string())
        .language("en".to_string())
        .keywords(vec![
            "keyword1".to_string(),
            "keyword2".to_string(),
            "keyword3".to_string(),
        ])
        .title("An awesome title".to_string())
        .authors(vec!["John Doe".to_string(), "Max Mustermann".to_string()])
        .text_direction(TextDirection::LeftToRight)
        .page_layout(PageLayout::TwoColumnRight);
    document.set_metadata(metadata);
}

#[snapshot(document)]
fn metadata_empty(document: &mut Document) {
    let metadata = Metadata::new();
    document.set_metadata(metadata);
}

#[snapshot(document)]
fn metadata_full(document: &mut Document) {
    metadata_impl(document);
}

#[snapshot(document, settings_5)]
fn metadata_full_with_xmp(document: &mut Document) {
    metadata_impl(document);
}

#[snapshot(document, settings_30)]
fn metadata_pdf_20_author(document: &mut Document) {
    let metadata = Metadata::new()
        .authors(vec!["John Doe".to_string(), "Max Mustermann".to_string()])
        .creation_date(datetime());
    document.set_metadata(metadata);
}

fn cc_namespace() -> Namespace {
    Namespace::new("cc", "http://creativecommons.org/ns#")
        .schema_name("Creative Commons")
        .add_description("license", "Text", Category::External, "License URL")
        .add_description(
            "attributionName",
            "Text",
            Category::External,
            "Attribution name",
        )
}

fn factur_x_namespace() -> Namespace {
    Namespace::new("fx", "urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#")
        .schema_name("Factur-X PDFA Extension Schema")
        .add_description(
            "DocumentType",
            "Text",
            Category::External,
            "Type of the embedded XML",
        )
        .add_description(
            "DocumentFileName",
            "Text",
            Category::External,
            "File name of the embedded XML",
        )
        .add_description(
            "Version",
            "Text",
            Category::External,
            "Version of the Factur-X profile",
        )
        .add_description(
            "ConformanceLevel",
            "Text",
            Category::External,
            "Conformance level of the invoice",
        )
}

#[snapshot(document, settings_5)]
fn metadata_custom_xmp_basic(document: &mut Document) {
    metadata_impl(document);

    let cc = cc_namespace();
    let mm = Namespace::new("xmpMM", "http://ns.adobe.com/xap/1.0/mm/");

    let metadata = Metadata::new()
        .creation_date(datetime())
        .custom_xmp_properties(vec![
            Property::new(
                cc.clone(),
                "license",
                Value::text("https://creativecommons.org/licenses/by/4.0/"),
            ),
            Property::new(
                cc.clone(),
                "attributionName",
                Value::LanguageAlternative(vec![
                    (None, "Krilla".to_string()),
                    (Some("de".to_string()), "Krilla (DE)".to_string()),
                ]),
            ),
            Property::new(
                cc.clone(),
                "tags",
                Value::UnorderedArray(vec![Value::text("a"), Value::text("b")]),
            ),
            Property::new(
                cc.clone(),
                "ordering",
                Value::OrderedArray(vec![Value::Integer(1), Value::Integer(2)]),
            ),
            Property::new(
                cc.clone(),
                "rating",
                Value::AlternativeArray(vec![Value::Real(4.5), Value::Real(5.0)]),
            ),
            Property::new(cc.clone(), "reviewed", Value::Bool(true)),
            Property::new(
                cc,
                "derivedFrom",
                Value::Struct(vec![
                    StructField::new(mm.clone(), "DocumentID", Value::text("uuid:source-doc")),
                    StructField::new(mm, "VersionID", Value::text("1.0")),
                ]),
            ),
        ]);
    document.set_metadata(metadata);
}

#[snapshot(document, settings_10)]
fn metadata_custom_xmp_factur_x(document: &mut Document) {
    let fx = factur_x_namespace();
    let metadata = Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_properties(vec![
            Property::new(fx.clone(), "DocumentType", Value::text("INVOICE")),
            Property::new(fx.clone(), "DocumentFileName", Value::text("factur-x.xml")),
            Property::new(fx.clone(), "Version", Value::text("1.0")),
            Property::new(fx, "ConformanceLevel", Value::text("BASIC")),
        ]);
    document.set_metadata(metadata);
}

/// `xmpDM` is a namespace xmp-writer serializes natively, but it is only a
/// predefined PDF/A schema from XMP 2005 on.
#[snapshot(document, settings_19)]
fn validate_pdf_a1_custom_xmp_builtin_namespace(document: &mut Document) {
    let dm = Namespace::new("xmpDM", "http://ns.adobe.com/xap/1.0/DynamicMedia/")
        .schema_name("XMP Dynamic Media")
        .add_description("scene", "Text", Category::External, "The name of the scene");
    let metadata = Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_properties(vec![Property::new(dm, "scene", Value::text("intro"))]);
    document.set_metadata(metadata);
}

/// A Factur-X invoice in the MINIMUM profile.
const FACTUR_X_MINIMUM_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rsm:CrossIndustryInvoice
    xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100"
    xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100"
    xmlns:udt="urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100">
  <rsm:ExchangedDocumentContext>
    <ram:BusinessProcessSpecifiedDocumentContextParameter>
      <ram:ID>A1</ram:ID>
    </ram:BusinessProcessSpecifiedDocumentContextParameter>
    <ram:GuidelineSpecifiedDocumentContextParameter>
      <ram:ID>urn:factur-x.eu:1p0:minimum</ram:ID>
    </ram:GuidelineSpecifiedDocumentContextParameter>
  </rsm:ExchangedDocumentContext>
  <rsm:ExchangedDocument>
    <ram:ID>2026-001</ram:ID>
    <ram:TypeCode>380</ram:TypeCode>
    <ram:IssueDateTime>
      <udt:DateTimeString format="102">20260615</udt:DateTimeString>
    </ram:IssueDateTime>
  </rsm:ExchangedDocument>
  <rsm:SupplyChainTradeTransaction>
    <ram:ApplicableHeaderTradeAgreement>
      <ram:BuyerReference>SERVEXEC</ram:BuyerReference>
      <ram:SellerTradeParty>
        <ram:Name>Krilla Seller GmbH</ram:Name>
        <ram:PostalTradeAddress>
          <ram:CountryID>DE</ram:CountryID>
        </ram:PostalTradeAddress>
        <ram:SpecifiedTaxRegistration>
          <ram:ID schemeID="VA">DE123456789</ram:ID>
        </ram:SpecifiedTaxRegistration>
      </ram:SellerTradeParty>
      <ram:BuyerTradeParty>
        <ram:Name>Buyer SARL</ram:Name>
      </ram:BuyerTradeParty>
    </ram:ApplicableHeaderTradeAgreement>
    <ram:ApplicableHeaderTradeDelivery/>
    <ram:ApplicableHeaderTradeSettlement>
      <ram:InvoiceCurrencyCode>EUR</ram:InvoiceCurrencyCode>
      <ram:SpecifiedTradeSettlementHeaderMonetarySummation>
        <ram:TaxBasisTotalAmount>100.00</ram:TaxBasisTotalAmount>
        <ram:TaxTotalAmount currencyID="EUR">19.00</ram:TaxTotalAmount>
        <ram:GrandTotalAmount>119.00</ram:GrandTotalAmount>
        <ram:DuePayableAmount>119.00</ram:DuePayableAmount>
      </ram:SpecifiedTradeSettlementHeaderMonetarySummation>
    </ram:ApplicableHeaderTradeSettlement>
  </rsm:SupplyChainTradeTransaction>
</rsm:CrossIndustryInvoice>
"#;

/// A Factur-X (ZUGFeRD) invoice: a PDF/A-3 carrying the Factur-X
/// XMP extension schema *and* the structured XML embedded.
#[snapshot(document, settings_10)]
fn metadata_custom_xmp_factur_x_embedded(document: &mut Document) {
    let fx = factur_x_namespace();
    let metadata = Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_properties(vec![
            Property::new(fx.clone(), "DocumentType", Value::text("INVOICE")),
            Property::new(fx.clone(), "DocumentFileName", Value::text("factur-x.xml")),
            Property::new(fx.clone(), "Version", Value::text("1.0")),
            Property::new(fx, "ConformanceLevel", Value::text("MINIMUM")),
        ]);
    document.set_metadata(metadata);

    document.embed_file(EmbeddedFile {
        path: "factur-x.xml".to_string(),
        mime_type: Some(MimeType::new("text/xml").unwrap()),
        description: Some("Factur-X invoice data".to_string()),
        association_kind: AssociationKind::Alternative,
        data: FACTUR_X_MINIMUM_XML.as_bytes().to_vec().into(),
        modification_date: Some(datetime()),
        compress: Some(false),
        location: None,
    });
}

#[test]
fn metadata_custom_xmp_pdf_a_missing_description() {
    let mut document = Document::new_with(settings_10());
    let undeclared = Namespace::new("fx", "urn:factur-x:test#");
    let metadata = Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_properties(vec![Property::new(
            undeclared,
            "DocumentType",
            Value::text("INVOICE"),
        )]);
    document.set_metadata(metadata);

    let result = document.finish();
    let Err(KrillaError::Validation(errors)) = result else {
        panic!("expected validation error, got {result:?}");
    };
    assert!(
        errors.iter().any(|(e, _)| matches!(
            e,
            ValidationError::MissingXmpPropertyDescription { property_name, namespace_uri }
                if property_name == "DocumentType" && namespace_uri == "urn:factur-x:test#"
        )),
        "expected MissingXmpPropertyDescription, got {errors:?}",
    );
}

#[test]
fn metadata_custom_xmp_predefined_namespace_pdf_a() {
    let mut document = Document::new_with(settings_10());
    // Properties in predefined schemas don't need an extension schema, so
    // no property descriptions are required even in PDF/A.
    let xmp_rights = Namespace::new("xmpRights", "http://ns.adobe.com/xap/1.0/rights/");
    let metadata = Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_properties(vec![Property::new(xmp_rights, "Marked", Value::Bool(true))]);
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn metadata_custom_xmp_exif_predefined_pdf_a() {
    let mut document = Document::new_with(settings_10());
    // EXIF is a predefined PDF/A schema even though xmp-writer has no native
    // variant for it, so it needs no extension schema and no descriptions.
    let exif = Namespace::new("exif", "http://ns.adobe.com/exif/1.0/");
    let metadata = Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_properties(vec![Property::new(exif, "ColorSpace", Value::Integer(1))]);
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn metadata_custom_xmp_camera_raw_version_specific() {
    // Camera Raw is predefined only from XMP 2005 on. It is exempt from an
    // extension schema in PDF/A-2/-3 but not in PDF/A-1.
    let camera_raw = || Namespace::new("crs", "http://ns.adobe.com/camera-raw-settings/1.0/");
    let metadata = || {
        Metadata::new()
            .creation_date(datetime())
            .language("en".to_string())
            .custom_xmp_properties(vec![Property::new(
                camera_raw(),
                "WhiteBalance",
                Value::text("Auto"),
            )])
    };

    // PDF/A-3 (XMP 2005): exempt, so no description required.
    let mut a3 = Document::new_with(settings_10());
    a3.set_metadata(metadata());
    assert!(a3.finish().is_ok());

    // PDF/A-1 (XMP 2004): not predefined, so a description is required.
    let mut a1 = Document::new_with(settings_19());
    a1.set_metadata(metadata());
    let result = a1.finish();
    let Err(KrillaError::Validation(errors)) = result else {
        panic!("expected validation error, got {result:?}");
    };
    assert!(
        errors.iter().any(|(e, _)| matches!(
            e,
            ValidationError::MissingXmpPropertyDescription { property_name, namespace_uri }
                if property_name == "WhiteBalance"
                    && namespace_uri == "http://ns.adobe.com/camera-raw-settings/1.0/"
        )),
        "expected MissingXmpPropertyDescription, got {errors:?}",
    );
}

#[test]
fn metadata_custom_xmp_conflicting_namespace() {
    let mut document = Document::new();
    let metadata = Metadata::new().custom_xmp_properties(vec![
        Property::new(
            cc_namespace(),
            "license",
            Value::text("https://creativecommons.org/licenses/by/4.0/"),
        ),
        Property::new(
            Namespace::new("cc", "http://creativecommons.org/ns#"),
            "attributionName",
            Value::text("Krilla"),
        ),
    ]);
    document.set_metadata(metadata);

    assert_eq!(
        document.finish(),
        Err(KrillaError::Xmp(XmpError::ConflictingNamespace(
            "http://creativecommons.org/ns#".to_string()
        )))
    );
}

#[test]
fn metadata_custom_xmp_conflicting_prefix() {
    let mut document = Document::new();
    let metadata = Metadata::new().custom_xmp_properties(vec![
        Property::new(
            Namespace::new("fx", "urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#"),
            "DocumentType",
            Value::text("INVOICE"),
        ),
        Property::new(
            Namespace::new("fx", "urn:factur-x:other#"),
            "Version",
            Value::text("1.0"),
        ),
    ]);
    document.set_metadata(metadata);

    assert_eq!(
        document.finish(),
        Err(KrillaError::Xmp(XmpError::ConflictingPrefix(
            "fx".to_string()
        )))
    );
}

#[test]
fn metadata_custom_xmp_reserved_prefix() {
    let mut document = Document::new();
    let metadata = Metadata::new().custom_xmp_properties(vec![Property::new(
        Namespace::new("dc", "http://example.com/custom#"),
        "something",
        Value::text("value"),
    )]);
    document.set_metadata(metadata);

    assert_eq!(
        document.finish(),
        Err(KrillaError::Xmp(XmpError::ReservedPrefix("dc".to_string())))
    );
}

#[test]
fn metadata_custom_xmp_non_finite_real() {
    let mut document = Document::new();
    let metadata = Metadata::new().custom_xmp_properties(vec![Property::new(
        cc_namespace(),
        "rating",
        Value::OrderedArray(vec![Value::Real(4.5), Value::Real(f64::NAN)]),
    )]);
    document.set_metadata(metadata);

    assert_eq!(
        document.finish(),
        Err(KrillaError::Xmp(XmpError::NonFiniteReal(
            "rating".to_string()
        )))
    );
}
