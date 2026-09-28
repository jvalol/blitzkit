pub const UNBOUNDED_F32: f32 = f32::INFINITY;

#[derive(Debug, Clone)]
pub struct RenderText {
    pub position: glam::Vec2,
    pub bounds: glam::Vec2,
    pub color: glam::Vec4,
    pub text: String,
    pub size: f32,
    /// Whether this line is one the player can choose. A title, a score and a
    /// label leave it false and are drawn untouched. See spec 0023.
    pub selectable: bool,
    pub focused: bool,
    pub centered: bool,
}

impl Default for RenderText {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0).into(),
            bounds: (UNBOUNDED_F32, UNBOUNDED_F32).into(),
            color: (1.0, 1.0, 1.0, 1.0).into(),
            text: String::new(),
            size: 16.0,
            selectable: false,
            focused: false,
            centered: false,
        }
    }
}

pub struct TextRenderer {
    pub render_texts: Vec<RenderText>,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            render_texts: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.render_texts.clear();
    }

    pub fn push_render_text(&mut self, text: RenderText) {
        self.render_texts.push(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_uses_glam_types() {
        let text = RenderText {
            position: glam::vec2(12.0, 34.0),
            color: glam::vec4(1.0, 0.0, 0.0, 1.0),
            ..Default::default()
        };

        assert_eq!(text.position, glam::Vec2::new(12.0, 34.0));
        assert_eq!(text.color.x, 1.0);
        // an unbounded default, so text is not clipped unless a game asks
        assert_eq!(text.bounds.x, UNBOUNDED_F32);
    }
}
