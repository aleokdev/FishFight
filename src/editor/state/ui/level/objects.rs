use std::ops::Deref;

use macroquad::prelude::collections::storage;

use crate::{
    editor::{
        actions::{UiAction, UiActionExt},
        state::{
            ui::level::{screen_to_world_pos, world_to_screen_pos},
            EditorTool, ObjectSettings, State,
        },
        util::{EguiCompatibleVec, EguiTextureHandler, MqCompatibleVec},
        view::LevelView,
    },
    map::{MapObject, MapObjectKind},
    AnimatedSpriteMetadata, Resources,
};

impl State {
    pub(super) fn draw_level_object_placement_overlay(
        &mut self,
        egui_ctx: &egui::Context,
        level_response: &egui::Response,
        painter: &egui::Painter,
        cursor_tile_pos: egui::Pos2,
    ) -> Option<UiAction> {
        let mut action = None;
        if let Some(settings) = &mut self.object_being_placed {
            let position =
                world_to_screen_pos(settings.position, level_response.rect.min, &self.level_view);

            let mut closed_window = false;

            egui::Window::new("Placing object")
                .fixed_pos(position)
                .collapsible(false)
                .resizable(false)
                .show(egui_ctx, |ui| {
                    egui::ComboBox::new("object_kind", "Kind")
                        .selected_text(format!("{}", settings.kind))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut settings.kind,
                                MapObjectKind::Decoration,
                                "Decoration",
                            )
                            .clicked()
                            .then(|| settings.id = None);
                            ui.selectable_value(
                                &mut settings.kind,
                                MapObjectKind::Environment,
                                "Environment",
                            )
                            .clicked()
                            .then(|| settings.id = None);
                            ui.selectable_value(&mut settings.kind, MapObjectKind::Item, "Item")
                                .clicked()
                                .then(|| settings.id = None);
                        });
                    egui::ComboBox::new("object_id", "Id")
                        .selected_text(settings.id.as_deref().unwrap_or("Pick one"))
                        .show_ui(ui, |ui| {
                            let resources = storage::get::<Resources>();

                            match settings.kind {
                                MapObjectKind::Item => resources.items.keys().for_each(|id| {
                                    ui.selectable_value(&mut settings.id, Some(id.clone()), id);
                                }),
                                MapObjectKind::Environment => {
                                    ["sproinger", "crab"].into_iter().for_each(|id| {
                                        ui.selectable_value(
                                            &mut settings.id,
                                            Some(id.to_owned()),
                                            id,
                                        );
                                    })
                                }
                                MapObjectKind::Decoration => {
                                    resources.decoration.keys().for_each(|id| {
                                        ui.selectable_value(&mut settings.id, Some(id.clone()), id);
                                    })
                                }
                            }
                        });

                    ui.horizontal(|ui| {
                        let create_button =
                            ui.add_enabled(settings.id.is_some(), egui::Button::new("Create"));
                        if create_button.clicked() {
                            action = Some(UiAction::CreateObject {
                                id: settings.id.as_ref().unwrap().clone(),
                                kind: settings.kind.clone(),
                                position: settings.position.into_macroquad(),
                                // TODO: Include layer id in object settings
                                layer_id: self.selected_layer.as_ref().unwrap().clone(),
                            });

                            closed_window = true;
                        }
                        if ui.button("Cancel").clicked() {
                            closed_window = true;
                        }
                    });
                });

            if closed_window {
                self.object_being_placed = None;
            }
        } else {
            if self.selected_tool == EditorTool::ObjectPlacer
                && level_response.clicked_by(egui::PointerButton::Primary)
            {
                let position = screen_to_world_pos(
                    egui_ctx.input().pointer.interact_pos().unwrap(),
                    level_response.rect.min.to_vec2(),
                    &self.level_view,
                );

                self.object_being_placed = Some(ObjectSettings {
                    position,
                    kind: MapObjectKind::Item,
                    id: None,
                });
            }
        }
        action
    }

    pub(super) fn draw_objects(
        &self,
        egui_ctx: &egui::Context,
        ui: &mut egui::Ui,
        response: &egui::Response,
        painter: &egui::Painter,
    ) -> Option<UiAction> {
        let mut action = None;

        self.map_resource
            .map
            .draw_order
            .iter()
            .filter_map(|layer_idx| self.map_resource.map.layers.get(layer_idx))
            .for_each(|layer| {
                action.then_do(self.draw_object_layer(layer, response, painter, ui, egui_ctx));
            });

        action
    }

    fn draw_object_layer(
        &self,
        layer: &crate::map::MapLayer,
        response: &egui::Response,
        painter: &egui::Painter,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
    ) -> Option<UiAction> {
        let mut action = None;
        let resources = storage::get::<Resources>();

        for (object_idx, object) in layer.objects.iter().enumerate() {
            let (dest, is_valid) =
                draw_object(object, response, &self.level_view, painter, &resources);

            let is_this_object_selected;
            let can_select_object;
            if let Some(selected_entity) = &self.selected_map_entity {
                is_this_object_selected = selected_entity.kind.is_object(&layer.id, object_idx);
                can_select_object = false;
            } else {
                is_this_object_selected = false;
                can_select_object = self.selected_tool == EditorTool::Cursor
                    && response.hovered()
                    && ui
                        .input()
                        .pointer
                        .hover_pos()
                        .map(|hover_pos| dest.contains(hover_pos))
                        .unwrap_or(false);
            }

            if can_select_object || is_this_object_selected {
                painter.add(egui::Shape::rect_stroke(
                    dest,
                    egui::Rounding::none(),
                    egui::Stroke::new(
                        1.,
                        if is_this_object_selected {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::GRAY
                        },
                    ),
                ));
            }

            if is_this_object_selected {
                if response.dragged() {
                    let cursor_offset = self.selected_map_entity.as_ref().unwrap().click_offset;
                    let cursor_level_pos = screen_to_world_pos(
                        ui.input().pointer.interact_pos().unwrap() + cursor_offset,
                        response.rect.min.to_vec2(),
                        &self.level_view,
                    );
                    action.then_do_some(UiAction::MoveObject {
                        index: object_idx,
                        layer_id: layer.id.clone(),
                        position: macroquad::math::Vec2::new(
                            cursor_level_pos.x,
                            cursor_level_pos.y,
                        ),
                    });
                } else if response.drag_released() {
                    action.then_do_some(UiAction::DeselectObject);
                }
            } else if can_select_object {
                self.show_object_info_tooltip(egui_ctx, object, is_valid);

                if response.drag_started() {
                    let click_pos = ui.input().pointer.interact_pos().unwrap();
                    action.then_do_some(UiAction::SelectObject {
                        index: object_idx,
                        layer_id: layer.id.clone(),
                        cursor_offset: dest.min - click_pos,
                    });
                }
            }
        }

        action
    }

    fn show_object_info_tooltip(
        &self,
        egui_ctx: &egui::Context,
        object: &MapObject,
        is_valid: bool,
    ) {
        egui::containers::show_tooltip_at_pointer(egui_ctx, egui::Id::new("object info"), |ui| {
            ui.set_max_width(200.);
            ui.vertical_centered(|ui| {
                ui.heading(&object.id);
                ui.label(egui::RichText::new(format!("{}", object.kind)).small());
            });
            ui.separator();
            ui.horizontal_top(|ui| {
                ui.label(egui::RichText::new("Position: ").weak());
                ui.label(
                    egui::RichText::new(format!("({}, {})", object.position.x, object.position.y))
                        .monospace(),
                );
            });
            if !is_valid {
                ui.label(
                    egui::RichText::new(
                        "Object is not valid (i.e. has no valid object or texture ID)",
                    )
                    .color(egui::Color32::RED),
                );
            }
        });
    }
}

fn draw_object(
    object: &crate::map::MapObject,
    response: &egui::Response,
    level_view: &LevelView,
    painter: &egui::Painter,
    resources: &impl Deref<Target = Resources>,
) -> (egui::Rect, bool) {
    const FULL_UV: egui::Rect = egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.));

    let draw_object = |texture_id: egui::TextureId,
                       offset: macroquad::math::Vec2,
                       dest_size: egui::Vec2,
                       uv: egui::Rect,
                       tint: egui::Color32|
     -> egui::Rect {
        let position_in_lvl = (object.position + offset).into_egui().to_pos2();

        let dest = egui::Rect::from_min_size(
            world_to_screen_pos(position_in_lvl, response.rect.min, level_view),
            dest_size,
        );

        let mut mesh = egui::Mesh::with_texture(texture_id);
        mesh.add_rect_with_uv(dest, uv, tint);
        painter.add(egui::Shape::mesh(mesh));
        dest
    };

    let draw_invalid_object = || -> egui::Rect {
        let texture_id = resources
            .textures
            .get("object_error_icon")
            .unwrap()
            .texture
            .egui_id();
        let dest_size = egui::vec2(32., 32.);
        let uv = egui::Rect::from_min_max(egui::pos2(0., 0.), egui::pos2(1., 1.));

        draw_object(
            texture_id,
            (0., 0.).into(),
            dest_size,
            uv,
            egui::Color32::WHITE,
        )
    };

    let draw_animated_sprite = |sprite: &AnimatedSpriteMetadata, row: Option<u32>| -> egui::Rect {
        if let Some(texture_res) = resources.textures.get(&sprite.texture_id) {
            let tint = sprite
                .tint
                .map(|color| {
                    let [r, g, b, a]: [u8; 4] = color.into();
                    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
                })
                .unwrap_or(egui::Color32::WHITE);

            let texture_id = texture_res.texture.egui_id();
            let texture_size =
                egui::vec2(texture_res.texture.width(), texture_res.texture.height());
            let frame_size = texture_res.frame_size().into_egui();

            let dest_size = sprite.scale.map(|s| s * frame_size).unwrap_or(frame_size);

            let uv = row
                .map(|row| {
                    egui::Rect::from_min_size(
                        (egui::vec2(0.0, row as f32 * frame_size.y) / texture_size).to_pos2(),
                        frame_size / texture_size,
                    )
                })
                .unwrap_or(FULL_UV);

            draw_object(texture_id, sprite.offset, dest_size, uv, tint)
        } else {
            // Invalid texture ID
            draw_invalid_object()
        }
    };

    let dest;
    let is_valid;
    match object.kind {
        MapObjectKind::Decoration => {
            if let Some(meta) = resources.decoration.get(&object.id) {
                dest = draw_animated_sprite(
                    &meta.sprite,
                    meta.sprite.animations.first().map(|a| a.row),
                );
                is_valid = true;
            } else {
                // Invalid object ID
                dest = draw_invalid_object();
                is_valid = false;
            }
        }

        MapObjectKind::Item => {
            if let Some(meta) = resources.items.get(&object.id) {
                dest = draw_animated_sprite(
                    &meta.sprite,
                    Some(
                        meta.sprite
                            .animations
                            .iter()
                            .find(|&a| a.id == *crate::player::IDLE_ANIMATION_ID)
                            .map(|a| a.row)
                            .unwrap_or_default(),
                    ),
                );
                is_valid = true;
            } else {
                // Invalid object ID
                dest = draw_invalid_object();
                is_valid = false;
            }
        }
        MapObjectKind::Environment => {
            if &object.id == "sproinger" {
                let texture_res = resources.textures.get("sproinger").unwrap();

                let texture_id = texture_res.texture.egui_id();
                let texture_size =
                    egui::vec2(texture_res.texture.width(), texture_res.texture.height());
                let dest_size = texture_res
                    .meta
                    .frame_size
                    .map(macroquad::math::Vec2::into_egui)
                    .unwrap_or_else(|| texture_size);
                let uv = egui::Rect::from_min_size(
                    egui::Vec2::ZERO.to_pos2(),
                    texture_res.frame_size().into_egui() / texture_size,
                );

                dest = draw_object(
                    texture_id,
                    (0., 0.).into(),
                    dest_size,
                    uv,
                    egui::Color32::WHITE,
                );
                is_valid = true;
            } else {
                // Invalid object ID
                dest = draw_invalid_object();
                is_valid = false;
            }
        }
    };

    (dest, is_valid)
}
