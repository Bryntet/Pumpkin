use crate::entity::Entity;
use crate::server::Server;
use pumpkin_core::math::vector3::Vector3;
use pumpkin_entity::entity_type::EntityType;
use pumpkin_entity::pose::EntityPose;
use pumpkin_protocol::uuid::UUID;
use pumpkin_world::item::ItemStack;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct ItemEntity {
    item_stack: ItemStack,
    is_able_to_be_picked_up: Arc<Mutex<bool>>,
    entity: Arc<Mutex<Entity>>,
}

impl ItemEntity {
    pub fn new(player_entity: &Entity, item_stack: ItemStack, server: &Server) -> Self {
        let is_able_to_be_picked_up = Arc::new(Mutex::new(false));
        let pick_up_clone = is_able_to_be_picked_up.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            *pick_up_clone.lock().await = true;
        });

        let pos = Vector3 {
            x: player_entity.pos.x,
            y: player_entity.pos.y + player_entity.standing_eye_height as f64 - 0.3,
            z: player_entity.pos.z,
        };

        let entity = Arc::new(Mutex::new(Entity {
            id: server.new_entity_id(),
            uuid: UUID(Uuid::new_v4()),
            entity_type: EntityType::Item,
            world: player_entity.world.clone(),
            pos,
            block_pos: player_entity.block_pos,
            chunk_pos: player_entity.chunk_pos,
            sneaking: false,
            sprinting: false,
            fall_flying: false,
            velocity: toss_velocity(player_entity),
            on_ground: false,
            yaw: 0.0,
            head_yaw: 0.0,
            pitch: 0.0,
            standing_eye_height: 0.0,
            pose: EntityPose::Standing,
        }));
        let entity_clone = entity.clone();
        tokio::spawn(async move { drop_loop(entity_clone).await });
        Self {
            item_stack,
            is_able_to_be_picked_up,
            entity,
        }
    }
}

async fn drop_loop(entity: Arc<Mutex<Entity>>) {
    loop {
        let mut entity = entity.lock().await;
        entity.advance_with_velocity();
        entity.apply_gravity();
    }
}

fn random_float() -> f64 {
    rand::thread_rng().gen_range(0.0..=1.0)
}

fn toss_velocity(player: &Entity) -> Vector3<f64> {
    use std::f64::consts::PI;
    let pitch_sin = f64::sin(player.pitch as f64 * (PI / 180.0));
    let pitch_cos = f64::cos(player.pitch as f64 * (PI / 180.0));
    let yaw_sin = f64::sin(player.yaw as f64 * (PI / 180.0));
    let yaw_cos = f64::cos(player.yaw as f64 * (PI / 180.0));
    let random_angle = random_float() * (2.0 * PI);
    let random_offset = 0.02 * random_float();

    Vector3 {
        x: (-yaw_sin * pitch_cos * 0.3) + f64::cos(random_angle) * random_offset,
        y: -pitch_sin * 0.3 + 0.1 + (random_float() - random_float()) * 0.1,
        z: (yaw_cos * pitch_cos * 0.3) + f64::sin(random_angle) * random_offset,
    }
}
