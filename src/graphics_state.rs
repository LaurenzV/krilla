use crate::object::ext_g_state;
use crate::object::ext_g_state::ExtGState;
use crate::util::TransformWrapper;
use tiny_skia_path::{Rect, Transform};

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct GraphicsState {
    ext_g_state: ExtGState,
    ctm: TransformWrapper,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ext_g_state: ExtGState::new(),
            ctm: TransformWrapper(Transform::identity()),
        }
    }
}

impl GraphicsState {
    pub fn combine(&mut self, other: &ext_g_state::ExtGState) {
        self.ext_g_state.combine(other);
    }

    pub fn concat_transform(&mut self, transform: Transform) {
        self.ctm = TransformWrapper(self.ctm.0.pre_concat(transform));
    }

    pub fn transform(&self) -> Transform {
        self.ctm.0
    }

    pub fn ext_g_state(&self) -> &ExtGState {
        &self.ext_g_state
    }
}

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
        bbox.transform(self.cur().transform()).unwrap()
    }
}
