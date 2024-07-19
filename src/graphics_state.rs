use crate::object::ext_g_state;
use crate::object::ext_g_state::ExtGState;
use tiny_skia_path::{Rect, Transform};

#[derive(Clone)]
struct GraphicsState {
    ext_g_state: ext_g_state::ExtGState,
    ctm: Transform,
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
    fn combine(&mut self, other: &ext_g_state::ExtGState) {
        self.ext_g_state.combine(other);
    }

    fn concat_transform(&mut self, transform: Transform) {
        self.ctm = self.ctm.pre_concat(transform);
        println!("result: {:?}", self.ctm);
    }

    fn transform(&self) -> Transform {
        self.ctm
    }
}

struct GraphicsStates {
    graphics_states: Vec<GraphicsState>,
}

impl GraphicsStates {
    fn new() -> Self {
        GraphicsStates {
            graphics_states: vec![GraphicsState::default()],
        }
    }

    fn cur(&self) -> &GraphicsState {
        self.graphics_states.last().unwrap()
    }

    fn cur_mut(&mut self) -> &mut GraphicsState {
        self.graphics_states.last_mut().unwrap()
    }

    fn save_state(&mut self) {
        let state = self.cur();
        self.graphics_states.push(state.clone())
    }

    fn restore_state(&mut self) {
        self.graphics_states.pop();
    }

    fn combine(&mut self, other: &ExtGState) {
        self.cur_mut().combine(other);
    }

    fn transform(&mut self, transform: Transform) {
        self.cur_mut().concat_transform(transform);
    }

    fn transform_bbox(&self, bbox: Rect) -> Rect {
        bbox.transform(self.cur().transform()).unwrap()
    }
}
