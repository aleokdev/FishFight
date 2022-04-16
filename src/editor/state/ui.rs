mod level;
mod side_panel;
mod toolbar;
mod windows;

use macroquad::prelude::RenderTarget;

use crate::editor::{
    actions::{UiAction, UiActionExt},
    view::LevelView,
};

impl super::State {
    pub fn ui(&mut self, egui_ctx: &egui::Context) -> Option<UiAction> {
        self.draw_toolbar(egui_ctx)
            .then(self.draw_side_panel(egui_ctx))
            .then(self.draw_level(egui_ctx))
            .then(self.draw_windows(egui_ctx))
    }
}
