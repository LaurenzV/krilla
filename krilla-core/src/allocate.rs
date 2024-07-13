/// Struct that keeps track name allocations in a XObject/Page.
#[derive(Clone, Copy, Eq, PartialEq, Default)]
pub struct ResourceNumberAllocator {
    /// The next number that will be used for the name of an XObject in a resource
    /// dictionary, e.g. "xo0".
    next_x_object_num: ResourceNumber,
    /// The next number that will be used for the name of a graphics state in a resource
    /// dictionary, e.g. "gs0".
    next_graphics_state_num: ResourceNumber,
    /// The next number that will be used for the name of a pattern in a resource
    /// dictionary, e.g. "po0".
    next_patterns_num: ResourceNumber,
    /// The next number that will be used for the name of a shading in a resource
    /// dictionary, e.g. "sh0".
    next_shadings_num: ResourceNumber,
    /// The next number that will be used for the name of a font in a resource
    /// dictionary, e.g. "fo0".
    next_fonts_num: ResourceNumber,
    /// The next number that will be used for the name of a color space in a resource
    /// dictionary, e.g. "cs0".
    next_color_space_num: ResourceNumber,
}

pub type ResourceNumber = u32;

impl ResourceNumberAllocator {
    /// Allocate a new XObject name.
    pub fn alloc_x_object_number(&mut self) -> ResourceNumber {
        let num = self.next_x_object_num;
        self.next_x_object_num.checked_add(1).unwrap();
        num
    }

    /// Allocate a new graphics state name.
    pub fn alloc_graphics_state_number(&mut self) -> ResourceNumber {
        let num = self.next_graphics_state_num;
        self.next_graphics_state_num.checked_add(1).unwrap();
        num
    }

    /// Allocate a new shading name.
    pub fn alloc_shading_number(&mut self) -> ResourceNumber {
        let num = self.next_shadings_num;
        self.next_shadings_num.checked_add(1).unwrap();
        num
    }

    /// Allocate a new shading name.
    pub fn alloc_font_number(&mut self) -> ResourceNumber {
        let num = self.next_fonts_num;
        self.next_fonts_num.checked_add(1).unwrap();
        num
    }

    /// Allocate a new color space name.
    pub fn alloc_color_space_number(&mut self) -> ResourceNumber {
        let num = self.next_color_space_num;
        self.next_color_space_num.checked_add(1).unwrap();
        num
    }
}