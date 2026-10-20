use std::sync::Arc;

use krilla::configure::ValidationError;
use krilla::embed::{AssociationKind, EmbeddedFile, MimeType};
use krilla::error::{KrillaError, KrillaResult};
use krilla::metadata::xmp::{
    Namespace, Property, PropertyDescription, Schema, SimpleType, StructType, Value, ValueType,
    XmpError,
};
use krilla::metadata::{DateTime, Metadata, PageLayout, TextDirection};
use krilla::Document;
use krilla_macros::snapshot;

use crate::{settings_10, settings_19};

pub(crate) fn datetime() -> DateTime {
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

#[snapshot(document, settings_5)]
fn metadata_version_id(document: &mut Document) {
    let metadata = Metadata::new().version_id("1.2.3".to_string());
    document.set_metadata(metadata);
}

#[snapshot(document, settings_19)]
fn metadata_version_id_pdf_a1(document: &mut Document) {
    let metadata = Metadata::new()
        .creation_date(datetime())
        .version_id("1.2.3".to_string());
    document.set_metadata(metadata);
}

#[snapshot(document)]
fn metadata_custom_fields(document: &mut Document) {
    document.set_metadata(custom_metadata());
}

pub(crate) fn custom_metadata() -> Metadata {
    Metadata::new()
        .title("Standard title".to_string())
        .custom_field("CustomField".to_string(), "old value".to_string())
        .unwrap()
        .custom_field("CustomField".to_string(), "new value".to_string())
        .unwrap()
        .custom_field("Custom/Key".to_string(), "Custom value".to_string())
        .unwrap()
}

#[snapshot(document, settings_30)]
fn metadata_pdf_20_author(document: &mut Document) {
    let metadata = Metadata::new()
        .authors(vec!["John Doe".to_string(), "Max Mustermann".to_string()])
        .creation_date(datetime());
    document.set_metadata(metadata);
}

fn text_description(name: &str, description: &str) -> PropertyDescription {
    PropertyDescription::new(name, SimpleType::Text, description)
}

fn cc_namespace() -> Namespace {
    Namespace::new("cc", "http://creativecommons.org/ns#")
}

fn factur_x_namespace() -> Namespace {
    Namespace::new("fx", "urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#")
}

fn factur_x_schema() -> Schema {
    Schema::new(factur_x_namespace())
        .name("Factur-X PDFA Extension Schema")
        .property(text_description("DocumentType", "Type of the embedded XML"))
        .property(text_description(
            "DocumentFileName",
            "File name of the embedded XML",
        ))
        .property(text_description(
            "Version",
            "Version of the Factur-X profile",
        ))
        .property(text_description(
            "ConformanceLevel",
            "Conformance level of the invoice",
        ))
}

fn factur_x_metadata(conformance_level: &str) -> Metadata {
    let fx = factur_x_namespace();
    Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_schema(factur_x_schema())
        .custom_xmp_property(fx.property("DocumentType", "INVOICE"))
        .custom_xmp_property(fx.property("DocumentFileName", "factur-x.xml"))
        .custom_xmp_property(fx.property("Version", "1.0"))
        .custom_xmp_property(fx.property("ConformanceLevel", conformance_level))
}

#[snapshot(document, settings_5)]
fn metadata_custom_xmp_basic(document: &mut Document) {
    metadata_impl(document);

    let cc = cc_namespace();
    let mm = Namespace::new("xmpMM", "http://ns.adobe.com/xap/1.0/mm/");

    let metadata = Metadata::new()
        .creation_date(datetime())
        .custom_xmp_property(cc.property("license", "https://creativecommons.org/licenses/by/4.0/"))
        .custom_xmp_property(cc.property(
            "attributionName",
            Value::LanguageAlternative(vec![
                (None, "Krilla".to_string()),
                (Some("de".to_string()), "Krilla (DE)".to_string()),
            ]),
        ))
        .custom_xmp_property(
            cc.property("tags", Value::UnorderedArray(vec!["a".into(), "b".into()])),
        )
        .custom_xmp_property(cc.property("ordering", Value::OrderedArray(vec![1.into(), 2.into()])))
        .custom_xmp_property(cc.property(
            "rating",
            Value::AlternativeArray(vec![4.5.into(), 5.0.into()]),
        ))
        .custom_xmp_property(cc.property("reviewed", true))
        .custom_xmp_property(cc.property(
            "derivedFrom",
            Value::Struct(vec![
                mm.property("DocumentID", "uuid:source-doc"),
                mm.property("VersionID", "1.0"),
            ]),
        ));
    document.set_metadata(metadata);
}

#[snapshot(document, settings_10)]
fn metadata_custom_xmp_factur_x(document: &mut Document) {
    document.set_metadata(factur_x_metadata("BASIC"));
}

#[snapshot(document, settings_10)]
fn metadata_custom_xmp_struct_type(document: &mut Document) {
    let ex = Namespace::new("ex", "http://example.com/xmp/address#");
    let address = Arc::new(
        StructType::new(ex.clone(), "Address", "A postal address")
            .field(text_description("street", "The street and house number"))
            .field(text_description("city", "The city")),
    );
    let schema = Schema::new(ex.clone())
        .name("Address book")
        .property(PropertyDescription::new(
            "addresses",
            ValueType::ordered_array(address.clone()),
            "Known addresses",
        ))
        .property(PropertyDescription::new(
            "ratio",
            SimpleType::Rational,
            "Aspect ratio",
        ));

    let metadata = Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_schema(schema)
        .custom_xmp_property(ex.property(
            "addresses",
            Value::OrderedArray(vec![Value::Struct(vec![
                ex.property("street", "Main Street 1"),
                ex.property("city", "Berlin"),
            ])]),
        ))
        .custom_xmp_property(ex.property("ratio", Value::Rational(16, 9)));
    document.set_metadata(metadata);
}

/// `xmpDM` is a namespace xmp-writer serializes natively, but it is only a
/// predefined PDF/A schema from XMP 2005 on.
#[snapshot(document, settings_19)]
fn validate_pdf_a1_custom_xmp_builtin_namespace(document: &mut Document) {
    let dm = Namespace::new("xmpDM", "http://ns.adobe.com/xmp/1.0/DynamicMedia/");
    let schema = Schema::new(dm.clone())
        .name("XMP Dynamic Media")
        .property(text_description("scene", "The name of the scene"));
    let metadata = Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_schema(schema)
        .custom_xmp_property(dm.property("scene", "intro"));
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
    document.set_metadata(factur_x_metadata("MINIMUM"));

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

fn pdf_a_metadata(property: Property) -> Metadata {
    Metadata::new()
        .creation_date(datetime())
        .language("en".to_string())
        .custom_xmp_property(property)
}

fn assert_missing_description(result: KrillaResult<Vec<u8>>, uri: &str, name: &str) {
    let Err(KrillaError::Validation(errors)) = result else {
        panic!("expected validation error, got {result:?}");
    };
    assert!(
        errors.iter().any(|(e, _)| matches!(
            e,
            ValidationError::MissingXmpPropertyDescription { property_name, namespace_uri }
                if property_name == name && namespace_uri == uri
        )),
        "expected MissingXmpPropertyDescription, got {errors:?}",
    );
}

#[test]
fn metadata_custom_xmp_pdf_a_missing_description() {
    let mut document = Document::new_with(settings_10());
    let undeclared = Namespace::new("fx", "urn:factur-x:test#");
    document.set_metadata(pdf_a_metadata(
        undeclared.property("DocumentType", "INVOICE"),
    ));

    assert_missing_description(document.finish(), "urn:factur-x:test#", "DocumentType");
}

#[test]
fn metadata_custom_xmp_no_schema_outside_pdf_a() {
    let mut document = Document::new();
    let tdm = Namespace::new("tdm", "http://www.w3.org/ns/tdmrep/");
    let metadata = Metadata::new()
        .custom_xmp_property(tdm.property("reservation", 1))
        .custom_xmp_property(tdm.property("policy", "https://example.com/policy.json"));
    document.set_metadata(metadata);

    assert!(document.finish().is_ok());
}

#[test]
fn metadata_custom_xmp_predefined_namespace_pdf_a() {
    let mut document = Document::new_with(settings_10());
    // Properties in predefined schemas don't need an extension schema, so
    // no property descriptions are required even in PDF/A.
    let xmp_rights = Namespace::new("xmpRights", "http://ns.adobe.com/xap/1.0/rights/");
    document.set_metadata(pdf_a_metadata(xmp_rights.property("Marked", true)));

    assert!(document.finish().is_ok());
}

#[test]
fn metadata_custom_xmp_exif_predefined_pdf_a() {
    let mut document = Document::new_with(settings_10());
    // EXIF is a predefined PDF/A schema even though xmp-writer has no native
    // variant for it, so it needs no extension schema and no descriptions.
    let exif = Namespace::new("exif", "http://ns.adobe.com/exif/1.0/");
    document.set_metadata(pdf_a_metadata(exif.property("ColorSpace", 1)));

    assert!(document.finish().is_ok());
}

#[test]
fn metadata_custom_xmp_camera_raw_version_specific() {
    // Camera Raw is predefined only from XMP 2005 on. It is exempt from an
    // extension schema in PDF/A-2/-3 but not in PDF/A-1.
    let crs = Namespace::new("crs", "http://ns.adobe.com/camera-raw-settings/1.0/");
    let metadata = || pdf_a_metadata(crs.property("WhiteBalance", "Auto"));

    let mut a3 = Document::new_with(settings_10());
    a3.set_metadata(metadata());
    assert!(a3.finish().is_ok());

    let mut a1 = Document::new_with(settings_19());
    a1.set_metadata(metadata());
    assert_missing_description(a1.finish(), crs.uri(), "WhiteBalance");
}

fn assert_xmp_error(metadata: Metadata, error: XmpError) {
    let mut document = Document::new();
    document.set_metadata(metadata);
    assert_eq!(document.finish(), Err(KrillaError::Xmp(error)));
}

#[test]
fn metadata_custom_xmp_conflicting_namespace() {
    assert_xmp_error(
        Metadata::new()
            .custom_xmp_property(cc_namespace().property("license", "CC-BY"))
            .custom_xmp_property(
                Namespace::new("cc2", "http://creativecommons.org/ns#")
                    .property("attributionName", "Krilla"),
            ),
        XmpError::ConflictingNamespace("http://creativecommons.org/ns#".to_string()),
    );
}

#[test]
fn metadata_custom_xmp_conflicting_prefix() {
    assert_xmp_error(
        Metadata::new()
            .custom_xmp_property(factur_x_namespace().property("DocumentType", "INVOICE"))
            .custom_xmp_property(
                Namespace::new("fx", "urn:factur-x:other#").property("Version", "1.0"),
            ),
        XmpError::ConflictingPrefix("fx".to_string()),
    );
}

#[test]
fn metadata_custom_xmp_reserved_prefix() {
    assert_xmp_error(
        Metadata::new().custom_xmp_property(
            Namespace::new("dc", "http://example.com/custom#").property("something", "value"),
        ),
        XmpError::ReservedPrefix("dc".to_string()),
    );
}

#[test]
fn metadata_custom_xmp_duplicate_schema() {
    assert_xmp_error(
        Metadata::new()
            .custom_xmp_schema(factur_x_schema())
            .custom_xmp_schema(Schema::new(factur_x_namespace())),
        XmpError::DuplicateSchema(factur_x_namespace().uri().to_string()),
    );
}

#[test]
fn metadata_custom_xmp_invalid_number() {
    assert_xmp_error(
        Metadata::new().custom_xmp_property(cc_namespace().property(
            "rating",
            Value::OrderedArray(vec![4.5.into(), f64::NAN.into()]),
        )),
        XmpError::InvalidNumber("rating".to_string()),
    );
    assert_xmp_error(
        Metadata::new()
            .custom_xmp_property(cc_namespace().property("ratio", Value::Rational(1, 0))),
        XmpError::InvalidNumber("ratio".to_string()),
    );
}

fn tdm_schema() -> (Namespace, Schema) {
    let tdm = Namespace::new("tdm", "http://www.w3.org/ns/tdmrep/");
    let schema = Schema::new(tdm.clone())
        .property(
            PropertyDescription::new(
                "reservation",
                ValueType::closed_choice(SimpleType::Integer, ["0", "1"]),
                "Whether TDM rights are reserved",
            )
            .required(),
        )
        .property(PropertyDescription::new(
            "policy",
            SimpleType::Uri,
            "A machine-readable TDM policy",
        ));
    (tdm, schema)
}

#[test]
fn metadata_custom_xmp_valid_against_schema() {
    let (tdm, schema) = tdm_schema();
    let mut document = Document::new();
    document.set_metadata(
        Metadata::new()
            .custom_xmp_schema(schema)
            .custom_xmp_property(tdm.property("reservation", 1))
            .custom_xmp_property(tdm.property("policy", "https://example.com/policy.json")),
    );

    assert!(document.finish().is_ok());
}

#[test]
fn metadata_custom_xmp_invalid_value() {
    let (tdm, schema) = tdm_schema();
    assert_xmp_error(
        Metadata::new()
            .custom_xmp_schema(schema.clone())
            .custom_xmp_property(tdm.property("reservation", 2)),
        XmpError::InvalidValue("reservation".to_string()),
    );
    assert_xmp_error(
        Metadata::new()
            .custom_xmp_schema(schema)
            .custom_xmp_property(tdm.property("reservation", 1))
            .custom_xmp_property(tdm.property("policy", true)),
        XmpError::InvalidValue("policy".to_string()),
    );
}

#[test]
fn metadata_custom_xmp_missing_required_property() {
    let (tdm, schema) = tdm_schema();
    assert_xmp_error(
        Metadata::new()
            .custom_xmp_schema(schema)
            .custom_xmp_property(tdm.property("policy", "https://example.com/policy.json")),
        XmpError::MissingProperty("reservation".to_string()),
    );
}

#[test]
fn metadata_custom_xmp_invalid_struct() {
    let ex = Namespace::new("ex", "http://example.com/xmp/address#");
    let address = StructType::new(ex.clone(), "Address", "A postal address")
        .field(text_description("street", "The street").required())
        .field(text_description("city", "The city"));
    let metadata = |fields| {
        Metadata::new()
            .custom_xmp_schema(Schema::new(ex.clone()).property(PropertyDescription::new(
                "address",
                address.clone(),
                "An address",
            )))
            .custom_xmp_property(ex.property("address", Value::Struct(fields)))
    };

    assert_xmp_error(
        metadata(vec![
            ex.property("street", "Main Street 1"),
            ex.property("country", "DE"),
        ]),
        XmpError::InvalidValue("address".to_string()),
    );
    assert_xmp_error(
        metadata(vec![ex.property("city", "Berlin")]),
        XmpError::MissingProperty("street".to_string()),
    );
}
