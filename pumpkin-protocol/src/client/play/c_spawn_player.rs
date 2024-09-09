use crate::{uuid::UUID, VarInt};
use pumpkin_core::math::vector3::Vector3;
use pumpkin_macros::packet;
use serde::Serialize;

#[derive(Serialize)]
#[packet(0x01)]
pub struct CSpawnEntity {
    pub entity_id: VarInt,
    pub entity_uuid: UUID,
    pub entity_type: VarInt,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pitch: u8,    // angle
    pub yaw: u8,      // angle
    pub head_yaw: u8, // angle
    pub data: VarInt,
    pub velocity_x: i16,
    pub velocity_y: i16,
    pub velocity_z: i16,
}
