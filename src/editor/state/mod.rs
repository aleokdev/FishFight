mod ui;

use crate::{
    map::{MapLayerKind, MapObjectKind},
    resources::MapResource,
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

pub enum SelectableEntityKind {
    Object { layer_id: String, index: usize },
    SpawnPoint { index: usize },
}

pub struct SelectableEntity {
    pub kind: SelectableEntityKind,
    pub click_offset: egui::Vec2,
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

#[derive(Debug)]
pub struct ObjectSettings {
    pub position: egui::Pos2,
    pub kind: MapObjectKind,
    pub id: Option<String>,
}

/// Contains the editor state, i.e. the data whose change is tracked by the [`ActionHistory`] of the
/// editor.
pub struct State {
    pub selected_tool: EditorTool,
    pub map_resource: MapResource,
    pub selected_layer: Option<String>,
    pub selected_tile: Option<TileSelection>,
    pub is_parallax_enabled: bool,
    pub should_draw_grid: bool,
    pub selected_map_entity: Option<SelectableEntity>,
    pub object_being_placed: Option<ObjectSettings>,
}

impl State {
    pub fn new(map_resource: MapResource) -> Self {
        Self {
            map_resource,
            selected_tool: EditorTool::Cursor,
            is_parallax_enabled: true,
            should_draw_grid: true,
            selected_layer: None,
            selected_tile: None,
            selected_map_entity: None,
            object_being_placed: None,
        }
    }

    pub fn selected_layer_type(&self) -> Option<MapLayerKind> {
        self.selected_layer
            .as_ref()
            .and_then(|id| self.map_resource.map.layers.get(id))
            .map(|layer| layer.kind)
    }
}
