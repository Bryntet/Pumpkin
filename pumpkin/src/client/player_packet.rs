use std::{f32::consts::PI, sync::Arc};

use crate::{
    commands::CommandSender,
    entity::player::{ChatMode, Hand, Player},
    server::Server,
    world::player_chunker,
};
use num_traits::FromPrimitive;
use pumpkin_config::ADVANCED_CONFIG;
use pumpkin_core::{
    math::{position::WorldPosition, wrap_degrees},
    text::TextComponent,
    GameMode,
};
use pumpkin_entity::EntityId;
use pumpkin_inventory::{InventoryError, WindowType};
use pumpkin_protocol::server::play::{SCloseContainer, SSetPlayerGround, SUseItem};
use pumpkin_protocol::{
    client::play::{
        Animation, CAcknowledgeBlockChange, CBlockUpdate, CEntityAnimation, CEntityVelocity,
        CHeadRot, CHurtAnimation, CPingResponse, CPlayerChatMessage, CUpdateEntityPos,
        CUpdateEntityPosRot, CUpdateEntityRot, CWorldEvent, FilterType,
    },
    server::play::{
        Action, ActionType, SChatCommand, SChatMessage, SClientInformationPlay, SConfirmTeleport,
        SInteract, SPlayPingRequest, SPlayerAction, SPlayerCommand, SPlayerPosition,
        SPlayerPositionRotation, SPlayerRotation, SSetCreativeSlot, SSetHeldItem, SSwingArm,
        SUseItemOn, Status,
    },
};
use pumpkin_world::block::{BlockFace, BlockId};
use pumpkin_world::global_registry;

use super::PlayerConfig;

fn modulus(a: f32, b: f32) -> f32 {
    ((a % b) + b) % b
}

/// Handles all Play Packets send by a real Player
/// NEVER TRUST THE CLIENT. HANDLE EVERY ERROR, UNWRAP/EXPECT ARE FORBIDDEN
impl Player {
    pub fn handle_confirm_teleport(
        &mut self,
        _server: &Arc<Server>,
        confirm_teleport: SConfirmTeleport,
    ) {
        if let Some((id, position)) = self.awaiting_teleport.as_ref() {
            if id == &confirm_teleport.teleport_id {
                // we should set the pos now to that we requested in the teleport packet, Is may fixed issues when the client sended position packets while being teleported
                self.entity.set_pos(position.x, position.y, position.z);

                self.awaiting_teleport = None;
            } else {
                self.kick(TextComponent::text("Wrong teleport id"))
            }
        } else {
            self.kick(TextComponent::text(
                "Send Teleport confirm, but we did not teleport",
            ))
        }
    }

    fn clamp_horizontal(pos: f64) -> f64 {
        pos.clamp(-3.0E7, 3.0E7)
    }

    fn clamp_vertical(pos: f64) -> f64 {
        pos.clamp(-2.0E7, 2.0E7)
    }

    pub async fn handle_position(&mut self, _server: &Arc<Server>, position: SPlayerPosition) {
        if position.x.is_nan() || position.feet_y.is_nan() || position.z.is_nan() {
            self.kick(TextComponent::text("Invalid movement"));
            return;
        }
        let entity = &mut self.entity;
        self.last_position = entity.pos;
        entity.set_pos(
            Self::clamp_horizontal(position.x),
            Self::clamp_vertical(position.feet_y),
            Self::clamp_horizontal(position.z),
        );
        entity.on_ground = position.ground;
        let on_ground = entity.on_ground;
        let entity_id = entity.id;
        let (x, y, z) = entity.pos.into();
        let (lastx, lasty, lastz) = self.last_position.into();
        let world = self.entity.world.clone();
        let world = world.lock().await;

        // let delta = Vector3::new(x - lastx, y - lasty, z - lastz);
        // let velocity = self.velocity;

        // // Player is falling down fast, we should account for that
        // let max_speed = if self.fall_flying { 300.0 } else { 100.0 };

        // teleport when more than 8 blocks (i guess 8 blocks)
        // TODO: REPLACE * 2.0 by movement packets. see vanilla for details
        // if delta.length_squared() - velocity.length_squared() > max_speed * 2.0 {
        //     self.teleport(x, y, z, self.entity.yaw, self.entity.pitch);
        //     return;
        // }
        // send new position to all other players
        world.broadcast_packet(
            &[self.client.token],
            &CUpdateEntityPos::new(
                entity_id.into(),
                (x * 4096.0 - lastx * 4096.0) as i16,
                (y * 4096.0 - lasty * 4096.0) as i16,
                (z * 4096.0 - lastz * 4096.0) as i16,
                on_ground,
            ),
        );
        player_chunker::update_position(&world, self).await;
    }

    pub async fn handle_position_rotation(
        &mut self,
        _server: &Arc<Server>,
        position_rotation: SPlayerPositionRotation,
    ) {
        if position_rotation.x.is_nan()
            || position_rotation.feet_y.is_nan()
            || position_rotation.z.is_nan()
        {
            self.kick(TextComponent::text("Invalid movement"));
            return;
        }
        if !position_rotation.yaw.is_finite() || !position_rotation.pitch.is_finite() {
            self.kick(TextComponent::text("Invalid rotation"));
            return;
        }
        let entity = &mut self.entity;

        self.last_position = entity.pos;
        entity.set_pos(
            Self::clamp_horizontal(position_rotation.x),
            Self::clamp_vertical(position_rotation.feet_y),
            Self::clamp_horizontal(position_rotation.z),
        );
        entity.on_ground = position_rotation.ground;
        entity.yaw = wrap_degrees(position_rotation.yaw) % 360.0;
        entity.pitch = wrap_degrees(position_rotation.pitch).clamp(-90.0, 90.0) % 360.0;

        let on_ground = entity.on_ground;
        let entity_id = entity.id;
        let (x, y, z) = entity.pos.into();
        let (lastx, lasty, lastz) = self.last_position.into();
        let yaw = modulus(entity.yaw * 256.0 / 360.0, 256.0);
        let pitch = modulus(entity.pitch * 256.0 / 360.0, 256.0);
        // let head_yaw = (entity.head_yaw * 256.0 / 360.0).floor();
        let world = self.entity.world.clone();
        let world = world.lock().await;

        // let delta = Vector3::new(x - lastx, y - lasty, z - lastz);
        // let velocity = self.velocity;

        // // Player is falling down fast, we should account for that
        // let max_speed = if self.fall_flying { 300.0 } else { 100.0 };

        // // teleport when more than 8 blocks (i guess 8 blocks)
        // // TODO: REPLACE * 2.0 by movement packets. see vanilla for details
        // if delta.length_squared() - velocity.length_squared() > max_speed * 2.0 {
        //     self.teleport(x, y, z, yaw, pitch);
        //     return;
        // }
        // send new position to all other players

        world.broadcast_packet(
            &[self.client.token],
            &CUpdateEntityPosRot::new(
                entity_id.into(),
                (x * 4096.0 - lastx * 4096.0) as i16,
                (y * 4096.0 - lasty * 4096.0) as i16,
                (z * 4096.0 - lastz * 4096.0) as i16,
                yaw as u8,
                pitch as u8,
                on_ground,
            ),
        );
        world.broadcast_packet(
            &[self.client.token],
            &CHeadRot::new(entity_id.into(), yaw as u8),
        );

        player_chunker::update_position(&world, self).await;
    }

    pub async fn handle_rotation(&mut self, _server: &Arc<Server>, rotation: SPlayerRotation) {
        if !rotation.yaw.is_finite() || !rotation.pitch.is_finite() {
            self.kick(TextComponent::text("Invalid rotation"));
            return;
        }
        let entity = &mut self.entity;
        entity.on_ground = rotation.ground;
        entity.yaw = wrap_degrees(rotation.yaw) % 360.0;
        entity.pitch = wrap_degrees(rotation.pitch).clamp(-90.0, 90.0) % 360.0;
        // send new position to all other players
        let on_ground = entity.on_ground;
        let entity_id = entity.id;
        let yaw = modulus(entity.yaw * 256.0 / 360.0, 256.0);
        let pitch = modulus(entity.pitch * 256.0 / 360.0, 256.0);
        // let head_yaw = modulus(entity.head_yaw * 256.0 / 360.0, 256.0);

        let world = self.entity.world.lock().await;
        let packet = CUpdateEntityRot::new(entity_id.into(), yaw as u8, pitch as u8, on_ground);
        // self.client.send_packet(&packet);
        world.broadcast_packet(&[self.client.token], &packet);
        let packet = CHeadRot::new(entity_id.into(), yaw as u8);
        //        self.client.send_packet(&packet);
        world.broadcast_packet(&[self.client.token], &packet);
    }

    pub fn handle_chat_command(&mut self, server: &Arc<Server>, command: SChatCommand) {
        let dispatcher = server.command_dispatcher.clone();
        dispatcher.handle_command(&mut CommandSender::Player(self), server, &command.command);
    }

    pub fn handle_player_ground(&mut self, _server: &Arc<Server>, ground: SSetPlayerGround) {
        self.entity.on_ground = ground.on_ground;
    }

    pub async fn handle_player_command(&mut self, _server: &Arc<Server>, command: SPlayerCommand) {
        if command.entity_id != self.entity.id.into() {
            return;
        }

        if let Some(action) = Action::from_i32(command.action.0) {
            match action {
                pumpkin_protocol::server::play::Action::StartSneaking => {
                    if !self.entity.sneaking {
                        self.entity.set_sneaking(&mut self.client, true).await
                    }
                }
                pumpkin_protocol::server::play::Action::StopSneaking => {
                    if self.entity.sneaking {
                        self.entity.set_sneaking(&mut self.client, false).await
                    }
                }
                pumpkin_protocol::server::play::Action::LeaveBed => todo!(),
                pumpkin_protocol::server::play::Action::StartSprinting => {
                    if !self.entity.sprinting {
                        self.entity.set_sprinting(&mut self.client, true).await
                    }
                }
                pumpkin_protocol::server::play::Action::StopSprinting => {
                    if self.entity.sprinting {
                        self.entity.set_sprinting(&mut self.client, false).await
                    }
                }
                pumpkin_protocol::server::play::Action::StartHorseJump => todo!(),
                pumpkin_protocol::server::play::Action::StopHorseJump => todo!(),
                pumpkin_protocol::server::play::Action::OpenVehicleInventory => todo!(),
                pumpkin_protocol::server::play::Action::StartFlyingElytra => {
                    let fall_flying = self.entity.check_fall_flying();
                    if self.entity.fall_flying != fall_flying {
                        self.entity
                            .set_fall_flying(&mut self.client, fall_flying)
                            .await;
                    }
                } // TODO
            }
        } else {
            self.kick(TextComponent::text("Invalid player command"))
        }
    }

    pub async fn handle_swing_arm(&mut self, _server: &Arc<Server>, swing_arm: SSwingArm) {
        match Hand::from_i32(swing_arm.hand.0) {
            Some(hand) => {
                let animation = match hand {
                    Hand::Main => Animation::SwingMainArm,
                    Hand::Off => Animation::SwingOffhand,
                };
                let id = self.entity_id();
                let world = self.entity.world.lock().await;
                world.broadcast_packet(
                    &[self.client.token],
                    &CEntityAnimation::new(id.into(), animation as u8),
                )
            }
            None => {
                self.kick(TextComponent::text("Invalid hand"));
            }
        };
    }

    pub async fn handle_chat_message(&mut self, _server: &Arc<Server>, chat_message: SChatMessage) {
        dbg!("got message");

        let message = chat_message.message;
        if message.len() > 256 {
            self.kick(TextComponent::text("Oversized message"));
            return;
        }

        // TODO: filter message & validation
        let gameprofile = &self.gameprofile;

        let world = self.entity.world.lock().await;
        world.broadcast_packet(
            &[self.client.token],
            &CPlayerChatMessage::new(
                pumpkin_protocol::uuid::UUID(gameprofile.id),
                1.into(),
                chat_message.signature.as_deref(),
                &message,
                chat_message.timestamp,
                chat_message.salt,
                &[],
                Some(TextComponent::text(&message)),
                FilterType::PassThrough,
                1.into(),
                TextComponent::text(&gameprofile.name.clone()),
                None,
            ),
        )

        /* server.broadcast_packet(
            self,
            &CDisguisedChatMessage::new(
                TextComponent::from(message.clone()),
                VarInt(0),
                gameprofile.name.clone().into(),
                None,
            ),
        ) */
    }

    pub fn handle_client_information_play(
        &mut self,
        _server: &Arc<Server>,
        client_information: SClientInformationPlay,
    ) {
        if let (Some(main_hand), Some(chat_mode)) = (
            Hand::from_i32(client_information.main_hand.into()),
            ChatMode::from_i32(client_information.chat_mode.into()),
        ) {
            self.config = PlayerConfig {
                locale: client_information.locale,
                view_distance: client_information.view_distance,
                chat_mode,
                chat_colors: client_information.chat_colors,
                skin_parts: client_information.skin_parts,
                main_hand,
                text_filtering: client_information.text_filtering,
                server_listing: client_information.server_listing,
            };
        } else {
            self.kick(TextComponent::text("Invalid hand or chat type"))
        }
    }

    pub async fn handle_interact(&mut self, _: &Arc<Server>, interact: SInteract) {
        let sneaking = interact.sneaking;
        if self.entity.sneaking != sneaking {
            self.entity.set_sneaking(&mut self.client, sneaking).await;
        }
        match ActionType::from_i32(interact.typ.0) {
            Some(action) => match action {
                ActionType::Attack => {
                    let entity_id = interact.entity_id;
                    // TODO: do validation and stuff
                    let config = &ADVANCED_CONFIG.pvp;
                    if config.enabled {
                        let world = self.entity.world.clone();
                        let world = world.lock().await;
                        let attacked_player = world.get_by_entityid(self, entity_id.0 as EntityId);
                        if let Some(mut player) = attacked_player {
                            let token = player.client.token;
                            let velo = player.entity.velocity;
                            if config.protect_creative && player.gamemode == GameMode::Creative {
                                return;
                            }
                            if config.knockback {
                                let yaw = self.entity.yaw;
                                let strength = 1.0;
                                player.entity.knockback(
                                    strength * 0.5,
                                    (yaw * (PI / 180.0)).sin() as f64,
                                    -(yaw * (PI / 180.0)).cos() as f64,
                                );
                                let packet = &CEntityVelocity::new(
                                    &entity_id,
                                    velo.x as f32,
                                    velo.y as f32,
                                    velo.z as f32,
                                );
                                self.entity.velocity = self.entity.velocity.multiply(0.6, 1.0, 0.6);

                                player.entity.velocity = velo;
                                player.client.send_packet(packet);
                            }
                            if config.hurt_animation {
                                // TODO
                                // thats how we prevent borrow errors :c
                                let packet = &CHurtAnimation::new(&entity_id, self.entity.yaw);
                                self.client.send_packet(packet);
                                player.client.send_packet(packet);
                                world.broadcast_packet(
                                    &[self.client.token, token],
                                    &CHurtAnimation::new(&entity_id, 10.0),
                                )
                            }
                            if config.swing {}
                        } else {
                            self.kick(TextComponent::text("Interacted with invalid entity id"))
                        }
                    }
                }
                ActionType::Interact => {
                    dbg!("todo");
                }
                ActionType::InteractAt => {
                    dbg!("todo");
                }
            },
            None => self.kick(TextComponent::text("Invalid action type")),
        }
    }
    pub async fn handle_player_action(
        &mut self,
        _server: &Arc<Server>,
        player_action: SPlayerAction,
    ) {
        match Status::from_i32(player_action.status.0) {
            Some(status) => match status {
                Status::StartedDigging => {
                    if !self.can_interact_with_block_at(&player_action.location, 1.0) {
                        // TODO: maybe log?
                        return;
                    }
                    // TODO: do validation
                    // TODO: Config
                    if self.gamemode == GameMode::Creative {
                        let location = player_action.location;
                        // Block break & block break sound
                        // TODO: currently this is always dirt replace it
                        let world = self.entity.world.lock().await;
                        world.broadcast_packet(
                            &[self.client.token],
                            &CWorldEvent::new(2001, &location, 11, false),
                        );
                        // AIR
                        world.broadcast_packet(
                            &[self.client.token],
                            &CBlockUpdate::new(&location, 0.into()),
                        );
                    }
                }
                Status::CancelledDigging => {
                    if !self.can_interact_with_block_at(&player_action.location, 1.0) {
                        // TODO: maybe log?
                        return;
                    }
                    self.current_block_destroy_stage = 0;
                }
                Status::FinishedDigging => {
                    // TODO: do validation
                    let location = player_action.location;
                    if !self.can_interact_with_block_at(&location, 1.0) {
                        // TODO: maybe log?
                        return;
                    }
                    // Block break & block break sound
                    // TODO: currently this is always dirt replace it
                    let world = self.entity.world.lock().await;
                    world.broadcast_packet(
                        &[self.client.token],
                        &CWorldEvent::new(2001, &location, 11, false),
                    );
                    // AIR
                    world.broadcast_packet(
                        &[self.client.token],
                        &CBlockUpdate::new(&location, 0.into()),
                    );
                    // TODO: Send this every tick
                    self.client
                        .send_packet(&CAcknowledgeBlockChange::new(player_action.sequence));
                }
                Status::DropItemStack => {
                    dbg!("todo");
                }
                Status::DropItem => {
                    dbg!("todo");
                }
                Status::ShootArrowOrFinishEating => {
                    dbg!("todo");
                }
                Status::SwapItem => {
                    dbg!("todo");
                }
            },
            None => self.kick(TextComponent::text("Invalid status")),
        }
    }

    pub fn handle_play_ping_request(&mut self, _server: &Arc<Server>, request: SPlayPingRequest) {
        self.client
            .send_packet(&CPingResponse::new(request.payload));
    }

    pub async fn handle_use_item_on(&mut self, _server: &Arc<Server>, use_item_on: SUseItemOn) {
        let location = use_item_on.location;

        if !self.can_interact_with_block_at(&location, 1.0) {
            // TODO: maybe log?
            return;
        }

        if let Some(face) = BlockFace::from_i32(use_item_on.face.0) {
            if let Some(item) = self.inventory.held_item() {
                let minecraft_id = global_registry::find_minecraft_id(
                    global_registry::ITEM_REGISTRY,
                    item.item_id,
                )
                .expect("All item ids are in the global registry");
                if let Ok(block_state_id) = BlockId::new(minecraft_id, None) {
                    let world = self.entity.world.lock().await;
                    world.broadcast_packet(
                        &[self.client.token],
                        &CBlockUpdate::new(&location, block_state_id.get_id_mojang_repr().into()),
                    );
                    world.broadcast_packet(
                        &[self.client.token],
                        &CBlockUpdate::new(
                            &WorldPosition(location.0 + face.to_offset()),
                            block_state_id.get_id_mojang_repr().into(),
                        ),
                    );
                }
            }
            self.client
                .send_packet(&CAcknowledgeBlockChange::new(use_item_on.sequence));
        } else {
            self.kick(TextComponent::text("Invalid block face"))
        }
    }

    pub fn handle_use_item(&mut self, _server: &Arc<Server>, _use_item: SUseItem) {
        // TODO: handle packet correctly
        log::error!("An item was used(SUseItem), but the packet is not implemented yet");
    }

    pub fn handle_set_held_item(&mut self, _server: &Arc<Server>, held: SSetHeldItem) {
        let slot = held.slot;
        if !(0..=8).contains(&slot) {
            self.kick(TextComponent::text("Invalid held slot"))
        }
        self.inventory.set_selected(slot as usize);
    }

    pub fn handle_set_creative_slot(
        &mut self,
        _server: &Arc<Server>,
        packet: SSetCreativeSlot,
    ) -> Result<(), InventoryError> {
        if self.gamemode != GameMode::Creative {
            return Err(InventoryError::PermissionError);
        }
        self.inventory
            .set_slot(packet.slot as usize, packet.clicked_item.to_item(), false)
    }

    // TODO:
    // This function will in the future be used to keep track of if the client is in a valid state.
    // But this is not possible yet
    pub fn handle_close_container(&mut self, server: &Arc<Server>, packet: SCloseContainer) {
        // window_id 0 represents both 9x1 Generic AND inventory here
        self.inventory.state_id = 0;
        if let Some(id) = self.open_container {
            let mut open_containers = server
                .open_containers
                .write()
                .expect("open_containers got poisoned");
            if let Some(container) = open_containers.get_mut(&id) {
                container.remove_player(self.entity_id())
            }
            self.open_container = None;
        }
        let Some(_window_type) = WindowType::from_u8(packet.window_id) else {
            self.kick(TextComponent::text("Invalid window ID"));
            return;
        };
    }
}
