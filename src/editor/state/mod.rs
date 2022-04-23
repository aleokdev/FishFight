mod ui;

use std::path::Path;

use macroquad::prelude::{collections::storage, RenderTarget};

use crate::{
    editor::actions,
    map::{Map, MapLayerKind, MapObjectKind},
    resources::{
        map_name_to_filename, MapResource, MAP_EXPORTS_DEFAULT_DIR, MAP_EXPORTS_EXTENSION,
    },
    Resources,
};

use super::{
    actions::UiAction, history::ActionHistory, input::EditorInput, view::LevelView, windows,
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EditorTool {
    Cursor,
    TilePlacer,
    ObjectPlacer,
    SpawnPointPlacer,
    Eraser,
}

pub struct TileSelection {
    pub tileset: String,
    pub tile_id: u32,
}

#[derive(Debug, Clone)]
pub enum SelectableEntityKind {
    Object { layer_id: String, index: usize },
    SpawnPoint { index: usize },
}

#[derive(Debug, Clone, Copy)]
pub struct DragData {
    pub cursor_offset: egui::Vec2,
    pub new_pos: egui::Pos2,
}

#[derive(Debug, Clone)]
pub struct SelectableEntity {
    pub kind: SelectableEntityKind,
    pub drag_data: Option<DragData>,
}

impl SelectableEntityKind {
    pub fn is_object(&self, layer_id: &str, index: usize) -> bool {
        match self {
            SelectableEntityKind::Object {
                layer_id: l,
                index: i,
            } => l == layer_id && i == &index,
            SelectableEntityKind::SpawnPoint { .. } => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectSettings {
    pub position: egui::Pos2,
    pub kind: MapObjectKind,
    pub id: Option<String>,
}

pub struct Editor {
    pub selected_tool: EditorTool,
    pub map_resource: MapResource,
    pub selected_layer: Option<String>,
    pub selected_tile: Option<TileSelection>,
    pub is_parallax_enabled: bool,
    pub should_draw_grid: bool,
    pub object_being_placed: Option<ObjectSettings>,

    create_layer_window: Option<windows::CreateLayerWindow>,
    create_tileset_window: Option<windows::CreateTilesetWindow>,
    pub menu_window: Option<windows::MenuWindow>,
    save_map_window: Option<windows::SaveMapWindow>,

    pub selection: Option<SelectableEntity>,

    history: ActionHistory,

    pub level_view: LevelView,

    pub level_render_target: RenderTarget,
}

impl Editor {
    pub fn new(map_resource: MapResource) -> Self {
        Self {
            map_resource,
            selected_tool: EditorTool::Cursor,
            is_parallax_enabled: true,
            should_draw_grid: true,
            selected_layer: None,
            selected_tile: None,
            object_being_placed: None,

            create_layer_window: None,
            create_tileset_window: None,
            menu_window: None,
            save_map_window: None,

            selection: None,

            history: ActionHistory::new(),

            level_view: LevelView {
                position: Default::default(),
                scale: 1.,
            },

            level_render_target: macroquad::prelude::render_target(1, 1),
        }
    }

    pub fn selected_layer_type(&self) -> Option<MapLayerKind> {
        self.selected_layer
            .as_ref()
            .and_then(|id| self.map_resource.map.layers.get(id))
            .map(|layer| layer.kind)
    }

    pub fn apply_action(&mut self, action: UiAction) {
        dbg!(&action);

        match action {
            UiAction::Batch(batch) => batch
                .into_iter()
                .for_each(|action| self.apply_action(action)),
            UiAction::Undo => self.history.undo(&mut self.map_resource.map).unwrap(),
            UiAction::Redo => self.history.redo(&mut self.map_resource.map).unwrap(),
            UiAction::SelectTool(tool) => self.selected_tool = tool,
            UiAction::CreateLayer {
                id,
                kind,
                has_collision,
                index,
            } => {
                let action = actions::CreateLayer::new(id, kind, has_collision, index);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
            }
            UiAction::CreateTileset { id, texture_id } => {
                let action = actions::CreateTileset::new(id, texture_id);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
            }
            UiAction::DeleteLayer(id) => {
                let action = actions::DeleteLayer::new(id);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
                self.selected_layer = None;
            }
            UiAction::DeleteTileset(id) => {
                let action = actions::DeleteTileset::new(id);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
                self.selected_tile = None;
            }
            UiAction::UpdateLayer { id, is_visible } => {
                let action = actions::UpdateLayer::new(id, is_visible);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
            }
            UiAction::SetLayerDrawOrderIndex { id, index } => {
                let action = actions::SetLayerDrawOrderIndex::new(id, index);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
            }
            UiAction::SelectLayer(id) => {
                if self.map_resource.map.layers[&id].kind == MapLayerKind::ObjectLayer
                    && self.selected_tool == EditorTool::TilePlacer
                {
                    self.selected_tool = EditorTool::Cursor;
                }
                self.selected_layer = Some(id);
            }
            UiAction::SelectTileset(id) => {
                self.selected_tile = Some(TileSelection {
                    tileset: id,
                    tile_id: 0,
                });
            }
            UiAction::SelectTile { id, tileset_id } => {
                self.selected_tile = Some(TileSelection {
                    tileset: tileset_id,
                    tile_id: id,
                });
                self.selected_tool = EditorTool::TilePlacer;
            }
            UiAction::PlaceTile {
                id,
                coords,
                layer_id,
                tileset_id,
            } => {
                let action = actions::PlaceTile::new(id, layer_id, tileset_id, coords);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
            }
            UiAction::RemoveTile { coords, layer_id } => {
                let action = actions::RemoveTile::new(layer_id, coords);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
            }
            UiAction::MoveObject {
                index,
                layer_id,
                position,
            } => {
                let action = actions::MoveObject::new(layer_id, index, position);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
            }
            UiAction::SelectEntity(selection) => {
                if let SelectableEntity {
                    kind: SelectableEntityKind::Object { layer_id, .. },
                    ..
                } = &selection
                {
                    self.selected_layer = Some(layer_id.clone());
                }
                self.selection = Some(selection);
            }
            UiAction::SaveMap { name } => {
                let mut map_resource = self.map_resource.clone();

                if let Some(name) = name {
                    let path = Path::new(MAP_EXPORTS_DEFAULT_DIR)
                        .join(map_name_to_filename(&name))
                        .with_extension(MAP_EXPORTS_EXTENSION);

                    map_resource.meta.name = name;
                    map_resource.meta.path = path.to_string_lossy().to_string();
                }

                map_resource.meta.is_user_map = true;
                map_resource.meta.is_tiled_map = false;

                let mut resources = storage::get_mut::<Resources>();
                if resources.save_map(&map_resource).is_ok() {
                    self.map_resource = map_resource;
                }
            }
            UiAction::CreateObject {
                id,
                kind,
                layer_id,
                position,
            } => {
                let action = actions::CreateObject::new(id, kind, position, layer_id);
                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
            }
            UiAction::DeleteObject { index, layer_id } => {
                let action = actions::DeleteObject::new(index, layer_id);

                self.history
                    .apply(action, &mut self.map_resource.map)
                    .unwrap();
            }
            UiAction::DeselectObject => {
                self.selection = None;
            }

            _ => todo!(),
        }
    }

    pub fn draw(&self) {
        let map = &self.map_resource.map;
        {
            map.draw_background(None, !self.is_parallax_enabled);
            map.draw(None, false);
        }

        if self.should_draw_grid {
            self::draw_grid(map);
        }
    }

    pub fn process_input(&mut self, input: &EditorInput) {
        const CAMERA_PAN_SPEED: f32 = 5.0;

        // Move camera
        {
            use macroquad::prelude::*;
            let target_size = vec2(
                self.level_render_target.texture.width(),
                self.level_render_target.texture.height(),
            );
            let zoom = vec2(
                self.level_view.scale / target_size.x,
                self.level_view.scale / target_size.y,
            ) * 2.;
            self.level_view.position += input.camera_move_direction * CAMERA_PAN_SPEED;
            let camera = Some(Camera2D {
                offset: vec2(-1., -1.),
                target: self.level_view.position,
                zoom,
                render_target: Some(self.level_render_target),
                ..Camera2D::default()
            });

            scene::set_camera(0, camera);
        }

        // Undo/redo
        if input.undo {
            self.apply_action(UiAction::Undo);
        }
        if input.redo {
            self.apply_action(UiAction::Redo);
        }

        // Toggle menu
        if input.toggle_menu {
            self.menu_window = if self.menu_window.is_some() {
                None
            } else {
                Some(Default::default())
            };
        }

        // Delete selected object
        if input.delete {
            if let Some(entity) = &self.selection {
                match &entity.kind {
                    SelectableEntityKind::Object { layer_id, index } => {
                        let layer_id = layer_id.clone();
                        let index = *index;
                        self.apply_action(UiAction::DeleteObject { layer_id, index });
                    }
                    SelectableEntityKind::SpawnPoint { index } => {
                        let index = *index;
                        self.apply_action(UiAction::DeleteSpawnPoint(index));
                    }
                }
                self.selection = None;
            }
        }
    }
}

fn draw_grid(map: &Map) {
    const GRID_LINE_WIDTH: f32 = 1.0;
    const GRID_COLOR: macroquad::prelude::Color = macroquad::prelude::Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 0.25,
    };

    use macroquad::prelude::*;

    let map_size = map.grid_size.as_f32() * map.tile_size;

    draw_rectangle_lines(
        map.world_offset.x,
        map.world_offset.y,
        map_size.x,
        map_size.y,
        GRID_LINE_WIDTH,
        GRID_COLOR,
    );

    for x in 0..map.grid_size.x {
        let begin = vec2(
            map.world_offset.x + (x as f32 * map.tile_size.x),
            map.world_offset.y,
        );

        let end = vec2(
            begin.x,
            begin.y + (map.grid_size.y as f32 * map.tile_size.y),
        );

        draw_line(begin.x, begin.y, end.x, end.y, GRID_LINE_WIDTH, GRID_COLOR)
    }

    for y in 0..map.grid_size.y {
        let begin = vec2(
            map.world_offset.x,
            map.world_offset.y + (y as f32 * map.tile_size.y),
        );

        let end = vec2(
            begin.x + (map.grid_size.x as f32 * map.tile_size.x),
            begin.y,
        );

        draw_line(begin.x, begin.y, end.x, end.y, GRID_LINE_WIDTH, GRID_COLOR)
    }
}