use pdf_writer::Name;

pub enum ContentTag {
    Span,
    Figure,
}

impl ContentTag {
    pub fn name(&self) -> Name {
        match self {
            ContentTag::Span => Name(b"Span"),
            ContentTag::Figure => Name(b"Figure"),
        }
    }
}

#[derive(Copy, Clone)]
pub enum MarkedContentIdentifier {
    Normal(usize, i32),
    Dummy,
}

impl MarkedContentIdentifier {
    pub fn new(page_index: usize) -> Self {
        Self::Normal(page_index, 0)
    }

    pub fn new_dummy() -> Self {
        Self::Dummy
    }

    pub fn bump(&mut self) -> MarkedContentIdentifier {
        let old = *self;

        match self {
            MarkedContentIdentifier::Normal(_, num) => {
                *num = num.checked_add(1).unwrap();
            }
            MarkedContentIdentifier::Dummy => {}
        }

        old
    }
}
