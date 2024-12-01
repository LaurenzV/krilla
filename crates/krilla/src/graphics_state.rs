//! PDF graphics state.

use std::hash::Hash;

use tiny_skia_path::{Rect, Transform};

use crate::object::ext_g_state::ExtGState;
use crate::util::HashExt;

/// A simulation of the PDF graphics state, so that we
/// can write our transforms/graphics state all at once
/// when adding an image/path instead of having to
/// use `save_state`/`restore_state` excessively.
#[derive(Debug, PartialEq, Clone)]
pub struct GraphicsState {
    ext_g_state: ExtGState,
    ctm: Transform,
}
impl Eq for GraphicsState {}

impl Hash for GraphicsState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ext_g_state.hash(state);
        self.ctm.hash(state);
    }
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ext_g_state: ExtGState::new(),
            ctm: Transform::identity(),
        }
    }
}

impl GraphicsState {
    pub fn combine(&mut self, other: &ExtGState) {
        self.ext_g_state.combine(other);
    }

    pub fn concat_transform(&mut self, transform: Transform) {
        self.ctm = self.ctm.pre_concat(transform);
    }

    pub fn transform(&self) -> Transform {
        self.ctm
    }

    pub fn ext_g_state(&self) -> &ExtGState {
        &self.ext_g_state
    }
}

/// A collection of graphics states, simulates the stack-like
/// structure used.
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct GraphicsStates {
    graphics_states: Vec<GraphicsState>,
}

impl GraphicsStates {
    pub fn new() -> Self {
        GraphicsStates {
            graphics_states: vec![GraphicsState::default()],
        }
    }

    pub fn cur(&self) -> &GraphicsState {
        self.graphics_states.last().unwrap()
    }

    pub fn cur_mut(&mut self) -> &mut GraphicsState {
        self.graphics_states.last_mut().unwrap()
    }

    pub fn save_state(&mut self) {
        let state = self.cur();
        self.graphics_states.push(state.clone())
    }

    pub fn restore_state(&mut self) {
        self.graphics_states.pop();
    }

    pub fn combine(&mut self, other: &ExtGState) {
        self.cur_mut().combine(other);
    }

    pub fn transform(&mut self, transform: Transform) {
        self.cur_mut().concat_transform(transform);
    }

    pub fn transform_bbox(&self, bbox: Rect) -> Rect {
        // Important: This does not take the root transform of the
        // corresponding ContentBuilder into account, because we
        // want it to be in krilla coordinates, not in PDF
        // coordinates.
        bbox.transform(self.cur().transform()).unwrap()
    }
}
