pub trait EguiCompatibleVec {
    fn to_egui(self) -> egui::Vec2;
}

impl EguiCompatibleVec for macroquad::math::Vec2 {
    fn to_egui(self) -> egui::Vec2 {
        egui::vec2(self.x, self.y)
    }
}

pub trait EguiTextureHandler {
    fn egui_id(&self) -> egui::TextureId;
}

impl EguiTextureHandler for macroquad::prelude::Texture2D {
    fn egui_id(&self) -> egui::TextureId {
        egui::TextureId::User(self.raw_miniquad_texture_handle().gl_internal_id() as u64)
    }
}

pub trait Resizable {
    /// Resizes by recreating the render target if the current width & height don't match the
    /// parameters given.
    fn resize_if_appropiate(&mut self, width: u32, height: u32);
}

impl Resizable for macroquad::prelude::RenderTarget {
    fn resize_if_appropiate(&mut self, width: u32, height: u32) {
        if width != self.texture.width() as u32 || height != self.texture.height() as u32 {
            self.delete();
            *self = macroquad::prelude::render_target(width, height);
        }
    }
}
