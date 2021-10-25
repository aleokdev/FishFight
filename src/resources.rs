use std::{collections::HashMap, fs, path::Path};

use macroquad::{
    audio::{load_sound, Sound},
    prelude::*,
};

use macroquad_particles::EmitterConfig;

use serde::{Deserialize, Serialize};

use crate::{
    error::{ErrorKind, Result},
    formaterr,
    items::ItemParams,
    json::{self, deserialize_bytes},
    map::Map,
};

#[derive(Serialize, Deserialize)]
struct ParticleEffectMetadata {
    id: String,
    path: String,
}

#[derive(Serialize, Deserialize)]
struct SoundMetadata {
    id: String,
    path: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureKind {
    Background,
    Tileset,
    Spritesheet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureMetadata {
    pub id: String,
    pub path: String,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<TextureKind>,
    #[serde(
        default,
        with = "json::uvec2_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub sprite_size: Option<UVec2>,
    #[serde(default = "json::default_filter_mode", with = "json::FilterModeDef")]
    pub filter_mode: FilterMode,
}

#[derive(Debug, Clone)]
pub struct TextureResource {
    pub texture: Texture2D,
    pub meta: TextureMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapMetadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub path: String,
    pub preview_path: String,
    #[serde(default, skip_serializing_if = "json::is_false")]
    pub is_tiled_map: bool,
    #[serde(default, skip_serializing_if = "json::is_false")]
    pub is_user_map: bool,
}

#[derive(Debug, Clone)]
pub struct MapResource {
    pub map: Map,
    pub preview: Texture2D,
    pub meta: MapMetadata,
}

pub struct Resources {
    pub assets_dir: String,

    pub particle_effects: HashMap<String, EmitterConfig>,
    pub sounds: HashMap<String, Sound>,
    pub music: HashMap<String, Sound>,
    pub textures: HashMap<String, TextureResource>,
    pub maps: Vec<MapResource>,
    pub items: HashMap<String, ItemParams>,
}

impl Resources {
    pub const PARTICLE_EFFECTS_DIR: &'static str = "particle_effects";
    pub const SOUNDS_FILE: &'static str = "sounds";
    pub const MUSIC_FILE: &'static str = "music";
    pub const TEXTURES_FILE: &'static str = "textures";
    pub const MAPS_FILE: &'static str = "maps";
    pub const ITEMS_FILE: &'static str = "items";

    pub const RESOURCE_FILES_EXTENSION: &'static str = "json";

    pub const MAP_EXPORTS_EXTENSION: &'static str = "ron";
    pub const MAP_EXPORTS_DEFAULT_DIR: &'static str = "maps";
    pub const MAP_PREVIEW_PLACEHOLDER_PATH: &'static str = "maps/no_preview.png";
    pub const MAP_PREVIEW_PLACEHOLDER_ID: &'static str = "map_preview_placeholder";

    pub async fn new(assets_dir: &str) -> Result<Resources> {
        let assets_dir_path = Path::new(assets_dir);

        let mut particle_effects = HashMap::new();

        {
            let particle_effects_file_path = assets_dir_path
                .join(Self::PARTICLE_EFFECTS_DIR)
                .with_extension(Self::RESOURCE_FILES_EXTENSION);

            let bytes = load_file(&particle_effects_file_path.to_string_lossy()).await?;
            let metadata: Vec<ParticleEffectMetadata> =
                deserialize_bytes(Self::RESOURCE_FILES_EXTENSION, &bytes)?;

            for meta in metadata {
                let file_path = assets_dir_path.join(&meta.path);

                let bytes = load_file(&file_path.to_string_lossy()).await?;
                let cfg: EmitterConfig = serde_json::from_slice(&bytes)?;

                particle_effects.insert(meta.id, cfg);
            }
        }

        let mut sounds = HashMap::new();

        {
            let sounds_file_path = assets_dir_path
                .join(Self::SOUNDS_FILE)
                .with_extension(Self::RESOURCE_FILES_EXTENSION);

            let bytes = load_file(&sounds_file_path.to_string_lossy()).await?;
            let metadata: Vec<SoundMetadata> =
                deserialize_bytes(Self::RESOURCE_FILES_EXTENSION, &bytes)?;

            for meta in metadata {
                let file_path = assets_dir_path.join(meta.path);

                let sound = load_sound(&file_path.to_string_lossy()).await?;

                sounds.insert(meta.id, sound);
            }
        }

        let mut music = HashMap::new();

        {
            let music_file_path = assets_dir_path
                .join(Self::MUSIC_FILE)
                .with_extension(Self::RESOURCE_FILES_EXTENSION);

            let bytes = load_file(&music_file_path.to_string_lossy()).await?;
            let metadata: Vec<SoundMetadata> =
                deserialize_bytes(Self::RESOURCE_FILES_EXTENSION, &bytes)?;

            for meta in metadata {
                let file_path = assets_dir_path.join(meta.path);

                let sound = load_sound(&file_path.to_string_lossy()).await?;

                music.insert(meta.id, sound);
            }
        }

        let mut textures = HashMap::new();

        {
            let textures_file_path = assets_dir_path
                .join(Self::TEXTURES_FILE)
                .with_extension(Self::RESOURCE_FILES_EXTENSION);

            let bytes = load_file(&textures_file_path.to_string_lossy()).await?;
            let metadata: Vec<TextureMetadata> =
                deserialize_bytes(Self::RESOURCE_FILES_EXTENSION, &bytes)?;

            for meta in metadata {
                let file_path = assets_dir_path.join(&meta.path);

                let texture = load_texture(&file_path.to_string_lossy()).await?;
                texture.set_filter(meta.filter_mode);

                let sprite_size = {
                    let val = meta
                        .sprite_size
                        .unwrap_or_else(|| vec2(texture.width(), texture.height()).as_u32());

                    Some(val)
                };

                let key = meta.id.clone();

                let res = TextureResource {
                    texture,
                    meta: TextureMetadata {
                        sprite_size,
                        ..meta
                    },
                };

                textures.insert(key, res);
            }
        }

        let mut maps = Vec::new();

        {
            let maps_file_path = assets_dir_path
                .join(Self::MAPS_FILE)
                .with_extension(Self::RESOURCE_FILES_EXTENSION);

            let bytes = load_file(&maps_file_path.to_string_lossy()).await?;
            let metadata: Vec<MapMetadata> =
                deserialize_bytes(Self::RESOURCE_FILES_EXTENSION, &bytes)?;

            for meta in metadata {
                let map_path = assets_dir_path.join(&meta.path);
                let preview_path = assets_dir_path.join(&meta.preview_path);

                let map = if meta.is_tiled_map {
                    Map::load_tiled(map_path, None).await?
                } else {
                    Map::load(map_path).await?
                };

                let preview = load_texture(&preview_path.to_string_lossy()).await?;

                let res = MapResource { map, preview, meta };

                maps.push(res)
            }
        }

        let mut items = HashMap::new();

        {
            let items_file_path = assets_dir_path
                .join(Self::ITEMS_FILE)
                .with_extension(Self::RESOURCE_FILES_EXTENSION);

            let bytes = load_file(&items_file_path.to_string_lossy()).await?;
            let item_paths: Vec<String> =
                deserialize_bytes(Self::RESOURCE_FILES_EXTENSION, &bytes)?;

            for path in item_paths {
                let path = assets_dir_path
                    .join(&path)
                    .with_extension(Self::RESOURCE_FILES_EXTENSION);

                let bytes = load_file(&path.to_string_lossy()).await?;

                let params: ItemParams = deserialize_bytes(Self::RESOURCE_FILES_EXTENSION, &bytes)?;

                items.insert(params.id.clone(), params);
            }
        }

        #[allow(clippy::inconsistent_struct_constructor)]
        Ok(Resources {
            assets_dir: assets_dir.to_string(),
            particle_effects,
            sounds,
            music,
            textures,
            maps,
            items,
        })
    }

    pub fn create_map(
        &mut self,
        name: &str,
        description: Option<&str>,
        tile_size: Vec2,
        grid_size: UVec2,
        should_overwrite: bool,
    ) -> Result<MapResource> {
        let description = description.map(|str| str.to_string());

        let assets_path = Path::new(&self.assets_dir);

        let map_path = Path::new(Self::MAP_EXPORTS_DEFAULT_DIR)
            .join(map_name_to_filename(name))
            .with_extension(Self::MAP_EXPORTS_EXTENSION);

        let path = map_path.to_string_lossy().into_owned();

        if assets_path.join(&map_path).exists() {
            let mut i = 0;
            while i < self.maps.len() {
                let res = &self.maps[i];
                if res.meta.path == path {
                    if res.meta.is_user_map && should_overwrite {
                        self.maps.remove(i);
                        break;
                    } else {
                        return Err(formaterr!(
                            ErrorKind::General,
                            "Resources: The path '{}' is in use and it is not possible to overwrite. Please choose a different map name",
                            map_path.to_str().unwrap(),
                        ));
                    }
                }

                i += 1;
            }
        }

        let preview_path = Path::new(Self::MAP_PREVIEW_PLACEHOLDER_PATH)
            .to_string_lossy()
            .into_owned();

        let meta = MapMetadata {
            name: name.to_string(),
            description,
            path,
            preview_path,
            is_tiled_map: false,
            is_user_map: true,
        };

        let map = Map::new(tile_size, grid_size);
        map.save(assets_path.join(map_path))?;

        let preview = {
            let res = self.textures.get(Self::MAP_PREVIEW_PLACEHOLDER_ID).unwrap();
            res.texture
        };

        let map_resource = MapResource { map, preview, meta };

        self.maps.push(map_resource.clone());

        let maps_file_path = assets_path
            .join(Self::MAPS_FILE)
            .with_extension(Self::RESOURCE_FILES_EXTENSION);

        let metadata: Vec<MapMetadata> = self.maps.iter().map(|res| res.meta.clone()).collect();

        let str = serde_json::to_string_pretty(&metadata)?;
        fs::write(maps_file_path, &str)?;

        Ok(map_resource)
    }
}

pub fn map_name_to_filename(name: &str) -> String {
    name.replace(' ', "_").replace('.', "_").to_lowercase()
}