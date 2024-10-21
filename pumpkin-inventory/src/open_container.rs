use crate::crafting::check_if_matches_crafting;
use crate::{Container, WindowType};
use pumpkin_world::item::ItemStack;
use std::sync::Arc;
use tokio::sync::Mutex;
pub struct OpenContainer {
    players: Vec<i32>,
    container: Arc<Mutex<Box<dyn Container>>>,
}

impl OpenContainer {
    pub fn try_open(&self, player_id: i32) -> Option<&Arc<Mutex<Box<dyn Container>>>> {
        if !self.players.contains(&player_id) {
            log::debug!("couldn't open container");
            return None;
        }
        let container = &self.container;
        Some(container)
    }

    pub fn add_player(&mut self, player_id: i32) {
        if !self.players.contains(&player_id) {
            self.players.push(player_id);
        }
    }

    pub fn remove_player(&mut self, player_id: i32) {
        if let Some(index) = self.players.iter().enumerate().find_map(|(index, id)| {
            if *id == player_id {
                Some(index)
            } else {
                None
            }
        }) {
            self.players.remove(index);
        }
    }

    pub fn new_empty_container<C: Container + Default + 'static>(player_id: i32) -> Self {
        Self {
            players: vec![player_id],
            container: Arc::new(Mutex::new(Box::new(C::default()))),
        }
    }

    pub fn all_player_ids(&self) -> Vec<i32> {
        self.players.clone()
    }
}
#[derive(Default)]
pub struct Chest([Option<ItemStack>; 27]);

impl Chest {
    pub fn new() -> Self {
        Self([None; 27])
    }
}
impl Container for Chest {
    fn window_type(&self) -> &'static WindowType {
        &WindowType::Generic9x3
    }

    fn window_name(&self) -> &'static str {
        "Chest"
    }
    fn all_slots(&mut self) -> Vec<&mut Option<ItemStack>> {
        self.0.iter_mut().collect()
    }

    fn all_slots_ref(&self) -> Vec<Option<&ItemStack>> {
        self.0.iter().map(|slot| slot.as_ref()).collect()
    }
}

#[derive(Default)]
pub struct CraftingTable {
    input: [[Option<ItemStack>; 3]; 3],
    output: Option<ItemStack>,
}

impl Container for CraftingTable {
    fn window_type(&self) -> &'static WindowType {
        &WindowType::CraftingTable
    }

    fn window_name(&self) -> &'static str {
        "Crafting Table"
    }
    fn all_slots(&mut self) -> Vec<&mut Option<ItemStack>> {
        let slots = vec![&mut self.output];
        let slots = slots
            .into_iter()
            .chain(self.input.iter_mut().flatten())
            .collect();
        slots
    }

    fn all_slots_ref(&self) -> Vec<Option<&ItemStack>> {
        let slots = vec![self.output.as_ref()];
        let slots = slots
            .into_iter()
            .chain(self.input.iter().flatten().map(|i| i.as_ref()))
            .collect();
        slots
    }

    fn craft(&mut self) -> bool {
        self.output = check_if_matches_crafting(self.input);
        self.input.iter().flatten().any(|s| s.is_some()) || self.output.is_some()
    }

    fn crafting_output_slot(&self) -> Option<usize> {
        Some(0)
    }

    fn recipe_used(&mut self) {
        self.input.iter_mut().flatten().for_each(|slot| {
            if let Some(item) = slot {
                if item.item_count > 1 {
                    item.item_count -= 1;
                } else {
                    *slot = None;
                }
            }
        })
    }

    fn all_combinable_slots(&self) -> Vec<Option<&ItemStack>> {
        self.input.iter().flatten().map(|s| s.as_ref()).collect()
    }
    fn all_combinable_slots_mut(&mut self) -> Vec<&mut Option<ItemStack>> {
        self.input.iter_mut().flatten().collect()
    }
}
