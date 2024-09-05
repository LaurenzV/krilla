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
pub enum ContentIdentifier {
    Normal(usize, i32),
    Dummy,
}

impl ContentIdentifier {
    pub fn new(page_index: usize) -> Self {
        Self::Normal(page_index, 0)
    }

    pub fn new_dummy() -> Self {
        Self::Dummy
    }

    pub fn bump(&mut self) -> ContentIdentifier {
        let old = *self;

        match self {
            ContentIdentifier::Normal(_, num) => {
                *num = num.checked_add(1).unwrap();
            }
            ContentIdentifier::Dummy => {}
        }

        old
    }
}

pub enum ContainerTag {
    Paragraph
}
