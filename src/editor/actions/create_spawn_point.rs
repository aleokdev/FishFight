use core::error::Result;

use macroquad::prelude::Vec2;

use crate::map::Map;

use super::{DeleteSpawnPoint, UndoableAction};

#[derive(Debug)]
pub struct CreateSpawnPoint {
    position: Vec2,
}

impl CreateSpawnPoint {
    pub fn new(position: Vec2) -> Self {
        CreateSpawnPoint { position }
    }
}

impl UndoableAction for CreateSpawnPoint {
    fn apply_to(&mut self, map: &mut Map) -> Result<Box<dyn UndoableAction>> {
        map.spawn_points.push(self.position);

        let inverse = Box::new(DeleteSpawnPoint::new(map.spawn_points.len() - 1));

        Ok(inverse)
    }

    fn undo(&mut self, map: &mut Map) -> Result<()> {
        map.spawn_points.pop();

        Ok(())
    }
}
