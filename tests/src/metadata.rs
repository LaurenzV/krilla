use krilla::metadata::{DateTime, Metadata, PageLayout, TextDirection};
use krilla::Document;
use krilla_macros::snapshot;

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
