use crate::{client::Client, world::World};
use num_traits::ToPrimitive;
use pumpkin_core::math::{
    get_section_cord, position::WorldPosition, vector2::Vector2, vector3::Vector3,
};
use pumpkin_entity::{entity_type::EntityType, pose::EntityPose, EntityId};
use pumpkin_protocol::client::play::CSpawnEntity;
use pumpkin_protocol::uuid::UUID;
use pumpkin_protocol::{
    client::play::{CSetEntityMetadata, Metadata},
    VarInt,
};
use std::sync::Arc;

pub mod item_entity;
pub mod player;

pub struct Entity {
    pub id: EntityId,
    pub uuid: UUID,
    pub entity_type: EntityType,
    pub world: Arc<tokio::sync::Mutex<World>>,

    pub pos: Vector3<f64>,
    pub block_pos: WorldPosition,
    pub chunk_pos: Vector2<i32>,

    pub sneaking: bool,
    pub sprinting: bool,
    pub fall_flying: bool,
    pub velocity: Vector3<f64>,

    // Should be not trusted
    pub on_ground: bool,

    pub yaw: f32,
    pub head_yaw: f32,
    pub pitch: f32,
    // TODO: Change this in diffrent poses
    pub standing_eye_height: f32,
    pub pose: EntityPose,
}

// TODO: Remove client: &mut Client, world: Arc<tokio::sync::Mutex<World>> bs
impl Entity {
    pub fn new(
        entity_id: EntityId,
        uuid: UUID,
        world: Arc<tokio::sync::Mutex<World>>,
        entity_type: EntityType,
        standing_eye_height: f32,
    ) -> Self {
        Self {
            id: entity_id,
            uuid,
            entity_type,
            on_ground: false,
            pos: Vector3::new(0.0, 0.0, 0.0),
            block_pos: WorldPosition(Vector3::new(0, 0, 0)),
            chunk_pos: Vector2::new(0, 0),
            sneaking: false,
            world,
            sprinting: false,
            fall_flying: false,
            yaw: 0.0,
            head_yaw: 0.0,
            pitch: 0.0,
            velocity: Vector3::new(0.0, 0.0, 0.0),
            standing_eye_height,
            pose: EntityPose::Standing,
        }
    }

    pub fn set_pos(&mut self, x: f64, y: f64, z: f64) {
        if self.pos.x != x || self.pos.y != y || self.pos.z != z {
            self.pos = Vector3::new(x, y, z);
            let i = x.floor() as i32;
            let j = y.floor() as i32;
            let k = z.floor() as i32;

            let block_pos = self.block_pos.0;
            if i != block_pos.x || j != block_pos.y || k != block_pos.z {
                self.block_pos = WorldPosition(Vector3::new(i, j, k));

                if get_section_cord(i) != self.chunk_pos.x
                    || get_section_cord(k) != self.chunk_pos.z
                {
                    self.chunk_pos =
                        Vector2::new(get_section_cord(block_pos.x), get_section_cord(block_pos.z));
                }
            }
        }
    }

    pub fn advance_with_velocity(&mut self) {
        let pos = self.pos;
        let velocity = self.velocity;
        self.set_pos(pos.x + velocity.x, pos.y + velocity.y, pos.z + velocity.z)
    }

    pub fn knockback(&mut self, strength: f64, x: f64, z: f64) {
        // This has some vanilla magic
        let mut x = x;
        let mut z = z;
        while x * x + z * z < 1.0E-5 {
            x = (rand::random::<f64>() - rand::random::<f64>()) * 0.01;
            z = (rand::random::<f64>() - rand::random::<f64>()) * 0.01;
        }

        let var8 = Vector3::new(x, 0.0, z).normalize() * strength;
        let var7 = self.velocity;
        self.velocity = Vector3::new(
            var7.x / 2.0 - var8.x,
            if self.on_ground {
                (var7.y / 2.0 + strength).min(0.4)
            } else {
                var7.y
            },
            var7.z / 2.0 - var8.z,
        );
    }

    pub async fn set_sneaking(&mut self, client: &mut Client, sneaking: bool) {
        assert!(self.sneaking != sneaking);
        self.sneaking = sneaking;
        self.set_flag(client, Self::SNEAKING_FLAG_INDEX, sneaking)
            .await;
        // if sneaking {
        //     self.set_pose(EntityPose::Crouching).await;
        // } else {
        //     self.set_pose(EntityPose::Standing).await;
        // }
    }

    pub async fn set_sprinting(&mut self, client: &mut Client, sprinting: bool) {
        assert!(self.sprinting != sprinting);
        self.sprinting = sprinting;
        self.set_flag(client, Self::SPRINTING_FLAG_INDEX, sprinting)
            .await;
    }

    pub fn check_fall_flying(&self) -> bool {
        !self.on_ground
    }

    pub async fn set_fall_flying(&mut self, client: &mut Client, fall_flying: bool) {
        assert!(self.fall_flying != fall_flying);
        self.fall_flying = fall_flying;
        self.set_flag(client, Self::FALL_FLYING_FLAG_INDEX, fall_flying)
            .await;
    }

    pub const ON_FIRE_FLAG_INDEX: u32 = 0;
    pub const SNEAKING_FLAG_INDEX: u32 = 1;
    pub const SPRINTING_FLAG_INDEX: u32 = 3;
    pub const SWIMMING_FLAG_INDEX: u32 = 4;
    pub const INVISIBLE_FLAG_INDEX: u32 = 5;
    pub const GLOWING_FLAG_INDEX: u32 = 6;
    pub const FALL_FLYING_FLAG_INDEX: u32 = 7;
    async fn set_flag(&mut self, client: &mut Client, index: u32, value: bool) {
        let mut b = 0i8;
        if value {
            b |= 1 << index;
        } else {
            b &= !(1 << index);
        }
        let packet = CSetEntityMetadata::new(self.id.into(), Metadata::new(0, 0.into(), b));
        client.send_packet(&packet);
        self.world
            .lock()
            .await
            .broadcast_packet(&[client.token], &packet);
    }

    pub async fn set_pose(&mut self, client: &mut Client, pose: EntityPose) {
        self.pose = pose;
        let pose = self.pose as i32;
        let packet = CSetEntityMetadata::<VarInt>::new(
            self.id.into(),
            Metadata::new(6, 20.into(), (pose).into()),
        );
        client.send_packet(&packet);
        self.world
            .lock()
            .await
            .broadcast_packet(&[client.token], &packet)
    }

    // This gets run once per "tick" (tokio task sleeping to imitate tick)
    pub fn apply_gravity(&mut self) {
        self.velocity.y -= self.entity_type.gravity()
    }
}

impl From<&Entity> for CSpawnEntity {
    fn from(entity: &Entity) -> Self {
        CSpawnEntity {
            entity_id: entity.id.into(),
            entity_uuid: entity.uuid.clone(),
            entity_type: VarInt::from(entity.entity_type as i32),
            x: entity.pos.x,
            y: entity.pos.y,
            z: entity.pos.z,
            pitch: entity
                .pitch
                .floor()
                .to_u8()
                .expect("Should be possible to convert pitch to u8"),
            yaw: entity
                .yaw
                .floor()
                .to_u8()
                .expect("Should be possible to convert yaw to u8"),
            head_yaw: entity
                .head_yaw
                .floor()
                .to_u8()
                .expect("Should be possible to convert head_yaw to u8"),
            data: VarInt(0),
            velocity_x: entity.velocity.x.floor() as i16,
            velocity_y: entity.velocity.y.floor() as i16,
            velocity_z: entity.velocity.z.floor() as i16,
        }
    }
}
