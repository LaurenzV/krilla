//! Setting document metadata.
//!
//! PDF allows for the inclusion of metadata in a PDF document. To do so in krilla,
//! you can simply create a [`Metadata`] object, set the data, and then include it
//! in the document via [`Document::set_metadata`].
//!
//! [`Document::set_metadata`]: crate::document::Document::set_metadata

use crate::serialize::SerializerContext;
use pdf_writer::{Pdf, Ref};
use xmp_writer::{Timezone, XmpWriter};

/// Metadata for a PDF document.
#[derive(Default, Clone)]
pub struct Metadata {
    pub(crate) title: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) creator: Option<String>,
    pub(crate) producer: Option<String>,
    pub(crate) keywords: Option<Vec<String>>,
    pub(crate) authors: Option<Vec<String>>,
    pub(crate) document_id: Option<String>,
    pub(crate) modification_date: Option<DateTime>,
    pub(crate) creation_date: Option<DateTime>,
}

impl Metadata {
    /// Create new metadata.
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// The title of the document.
    pub fn title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// The subject of the document.
    pub fn subject(mut self, subject: String) -> Self {
        self.subject = Some(subject);
        self
    }

    /// The keywords that describe the document.
    pub fn keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = Some(keywords);
        self
    }

    /// The creator tool of the document.
    pub fn creator(mut self, creator: String) -> Self {
        self.creator = Some(creator);
        self
    }

    /// The producer tool of the document.
    pub fn producer(mut self, producer: String) -> Self {
        self.producer = Some(producer);
        self
    }

    /// The authors of the document.
    pub fn authors(mut self, authors: Vec<String>) -> Self {
        self.authors = Some(authors);
        self
    }

    /// The creation date of the document.
    pub fn creation_date(mut self, creation_date: DateTime) -> Self {
        self.creation_date = Some(creation_date);
        self
    }

    /// A document ID.
    ///
    /// This attribute will be used as an identifier for identifying
    /// different versions of the same document.
    pub fn document_id(mut self, document_id: String) -> Self {
        self.document_id = Some(document_id);
        self
    }

    /// The modification date of the document.
    pub fn modification_date(mut self, modification_date: DateTime) -> Self {
        self.modification_date = Some(modification_date);
        self
    }

    pub(crate) fn has_document_info(&self) -> bool {
        self.title.is_some()
            || self.producer.is_some()
            || self.keywords.is_some()
            || self.authors.is_some()
            || self.creator.is_some()
            || self.modification_date.is_some()
            || self.creation_date.is_some()
            || self.subject.is_some()
    }

    pub(crate) fn serialize_xmp_metadata(&self, xmp: &mut XmpWriter) {
        if let Some(title) = &self.title {
            xmp.title([(None, title.as_str())]);
        }

        if let Some(subject) = &self.subject {
            xmp.subject([subject.as_str()]);
        }

        if let Some(keywords) = &self.keywords {
            let joined = keywords.join(", ");
            xmp.pdf_keywords(joined.as_str());
        }

        if let Some(authors) = &self.authors {
            // Turns out that if the authors are given in both the document
            // information dictionary and the XMP metadata, Acrobat takes a little
            // bit of both: The first author from the document information
            // dictionary and the remaining authors from the XMP metadata.
            //
            // To fix this for Acrobat, we could omit the remaining authors or all
            // metadata from the document information catalog (it is optional) and
            // only write XMP. However, not all other tools (including Apple
            // Preview) read the XMP data. This means we do want to include all
            // authors in the document information dictionary.
            //
            // Thus, the only alternative is to fold all authors into a single
            // `<rdf:li>` in the XMP metadata. This is, in fact, exactly what the
            // PDF/A spec Part 1 section 6.7.3 has to say about the matter. It's a
            // bit weird to not use the array (and it makes Acrobat show the author
            // list in quotes), but there's not much we can do about that.
            let joined = authors.join(", ");
            xmp.creator([joined.as_str()]);
        }

        if let Some(creator) = &self.creator {
            xmp.creator_tool(creator);
        }

        if let Some(producer) = &self.producer {
            xmp.producer(producer);
        }

        if let Some(date_time) = self.modification_date {
            xmp.modify_date(xmp_date(date_time));
        }

        if let Some(date_time) = self.creation_date {
            xmp.create_date(xmp_date(date_time));
        }
    }

    pub(crate) fn serialize_document_info(
        &self,
        ref_: &mut Ref,
        sc: &mut SerializerContext,
        pdf: &mut Pdf,
    ) {
        if self.has_document_info() {
            let ref_ = ref_.bump();
            let mut document_info = pdf.document_info(ref_);

            if let Some(title) = &self.title {
                document_info.title(sc.new_text_str(title));
            }

            if let Some(subject) = &self.subject {
                document_info.subject(sc.new_text_str(subject));
            }

            if let Some(keywords) = &self.keywords {
                let joined = keywords.join(", ");
                document_info.keywords(sc.new_text_str(&joined));
            }

            if let Some(authors) = &self.authors {
                let joined = authors.join(", ");
                document_info.author(sc.new_text_str(&joined));
            }

            if let Some(creator) = &self.creator {
                document_info.creator(sc.new_text_str(creator));
            }

            if let Some(producer) = &self.producer {
                document_info.producer(sc.new_text_str(producer));
            }

            if let Some(date_time) = self.modification_date {
                document_info.modified_date(pdf_date(date_time));
            }

            if let Some(date_time) = self.creation_date {
                document_info.creation_date(pdf_date(date_time));
            }
        }
    }
}

/// A datetime. Invalid values will be clamped.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct DateTime {
    /// The year (0-9999).
    pub(crate) year: u16,
    /// The month (0-11).
    pub(crate) month: Option<u8>,
    /// The month (0-30).
    pub(crate) day: Option<u8>,
    /// The hour (0-23).
    pub(crate) hour: Option<u8>,
    /// The minute (0-59).
    pub(crate) minute: Option<u8>,
    /// The second (0-59).
    pub(crate) second: Option<u8>,
    /// The hour offset from UTC (-23 through 23).
    pub(crate) utc_offset_hour: Option<i8>,
    /// The minute offset from UTC (0-59). Will carry over the sign from
    /// `utc_offset_hour`.
    pub(crate) utc_offset_minute: u8,
}

impl DateTime {
    /// Create a new, minimal date. The year will be clamped within the range
    /// 0-9999.
    #[inline]
    pub fn new(year: u16) -> Self {
        Self {
            year: year.min(9999),
            month: None,
            day: None,
            hour: None,
            minute: None,
            second: None,
            utc_offset_hour: None,
            utc_offset_minute: 0,
        }
    }

    /// Add the month field. It will be clamped within the range 1-12.
    #[inline]
    pub fn month(mut self, month: u8) -> Self {
        self.month = Some(month.clamp(1, 12));
        self
    }

    /// Add the day field. It will be clamped within the range 1-31.
    #[inline]
    pub fn day(mut self, day: u8) -> Self {
        self.day = Some(day.clamp(1, 31));
        self
    }

    /// Add the hour field. It will be clamped within the range 0-23.
    #[inline]
    pub fn hour(mut self, hour: u8) -> Self {
        self.hour = Some(hour.min(23));
        self
    }

    /// Add the minute field. It will be clamped within the range 0-59.
    #[inline]
    pub fn minute(mut self, minute: u8) -> Self {
        self.minute = Some(minute.min(59));
        self
    }

    /// Add the second field. It will be clamped within the range 0-59.
    #[inline]
    pub fn second(mut self, second: u8) -> Self {
        self.second = Some(second.min(59));
        self
    }

    /// Add the offset from UTC in hours. If not specified, the time will be
    /// assumed to be local to the viewer's time zone. It will be clamped within
    /// the range -23-23.
    #[inline]
    pub fn utc_offset_hour(mut self, hour: i8) -> Self {
        self.utc_offset_hour = Some(hour.clamp(-23, 23));
        self
    }

    /// Add the offset from UTC in minutes. This will have the same sign as set in
    /// [`Self::utc_offset_hour`]. It will be clamped within the range 0-59.
    #[inline]
    pub fn utc_offset_minute(mut self, minute: u8) -> Self {
        self.utc_offset_minute = minute.min(59);
        self
    }
}

/// Converts a datetime to a pdf-writer date.
fn pdf_date(date_time: DateTime) -> pdf_writer::Date {
    let mut pdf_date = pdf_writer::Date::new(date_time.year);

    if let Some(month) = date_time.month {
        pdf_date = pdf_date.month(month);
    }

    if let Some(day) = date_time.day {
        pdf_date = pdf_date.day(day);
    }

    if let Some(h) = date_time.hour {
        pdf_date = pdf_date.hour(h);
    }

    if let Some(m) = date_time.minute {
        pdf_date = pdf_date.minute(m);
    }

    if let Some(s) = date_time.second {
        pdf_date = pdf_date.second(s);
    }

    if let Some(oh) = date_time.utc_offset_hour {
        pdf_date = pdf_date.utc_offset_hour(oh);
    }

    pdf_date = pdf_date.utc_offset_minute(date_time.utc_offset_minute);

    pdf_date
}

/// Converts a datetime to an xmp-writer datetime.
fn xmp_date(datetime: DateTime) -> xmp_writer::DateTime {
    let timezone = match (datetime.utc_offset_hour, datetime.utc_offset_minute) {
        (None, _) => Some(Timezone::Utc),
        (Some(0), 0) => Some(Timezone::Utc),
        (Some(h), m) => {
            if let Ok(minute) = i8::try_from(m) {
                Some(Timezone::Local { hour: h, minute })
            } else {
                None
            }
        }
    };

    xmp_writer::DateTime {
        year: datetime.year,
        month: datetime.month,
        day: datetime.day,
        hour: datetime.hour,
        minute: datetime.minute,
        second: datetime.second,
        timezone,
    }
}
