//! A mod inventory's slots, optionally aliased by multiple block entities.
//!
//! Aliases share the actual inventory, so two hoppers cannot both extract a
//! snapshot of the same last item. The world keeps the channel alive when its
//! last endpoint unloads. Local inventories retain value semantics on clone.
use crate::items::ItemStack;
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

type Cells = Vec<Option<ItemStack>>;

#[derive(Debug)]
pub struct Inventory {
    channel: Option<String>,
    cells: Arc<Mutex<Cells>>,
}

impl From<Cells> for Inventory {
    fn from(cells: Cells) -> Self {
        Self {
            channel: None,
            cells: Arc::new(Mutex::new(cells)),
        }
    }
}

impl Clone for Inventory {
    fn clone(&self) -> Self {
        if self.channel.is_some() {
            Self {
                channel: self.channel.clone(),
                cells: self.cells.clone(),
            }
        } else {
            self.read().to_vec().into()
        }
    }
}

impl Inventory {
    pub fn shared(channel: String, cells: Cells) -> Self {
        Self {
            channel: Some(channel),
            cells: Arc::new(Mutex::new(cells)),
        }
    }
    pub fn channel(&self) -> Option<&str> {
        self.channel.as_deref()
    }
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cells, &other.cells)
    }
    pub fn read(&self) -> MutexGuard<'_, Cells> {
        self.cells
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub fn len(&self) -> usize {
        self.read().len()
    }
    pub fn is_empty(&self) -> bool {
        self.read().is_empty()
    }
}

pub enum Slots<'a> {
    Local(&'a [Option<ItemStack>]),
    Shared(MutexGuard<'a, Cells>),
}
impl Deref for Slots<'_> {
    type Target = [Option<ItemStack>];
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Local(cells) => cells,
            Self::Shared(cells) => cells,
        }
    }
}

pub enum SlotsMut<'a> {
    Local(&'a mut [Option<ItemStack>]),
    Shared(MutexGuard<'a, Cells>),
}
impl Deref for SlotsMut<'_> {
    type Target = [Option<ItemStack>];
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Local(cells) => cells,
            Self::Shared(cells) => cells,
        }
    }
}
impl DerefMut for SlotsMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Local(cells) => cells,
            Self::Shared(cells) => cells,
        }
    }
}
impl<'a> SlotsMut<'a> {
    pub fn slot(self, index: usize) -> Option<SlotMut<'a>> {
        (index < self.len()).then_some(SlotMut { slots: self, index })
    }
}

pub struct SlotMut<'a> {
    slots: SlotsMut<'a>,
    index: usize,
}
impl<'a> From<&'a mut Option<ItemStack>> for SlotMut<'a> {
    fn from(cell: &'a mut Option<ItemStack>) -> Self {
        Self {
            slots: SlotsMut::Local(std::slice::from_mut(cell)),
            index: 0,
        }
    }
}
impl Deref for SlotMut<'_> {
    type Target = Option<ItemStack>;
    fn deref(&self) -> &Self::Target {
        &self.slots[self.index]
    }
}
impl DerefMut for SlotMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slots[self.index]
    }
}

/// World-owned canonical channels. Endpoint lifetimes do not own their contents.
#[derive(Default)]
pub struct SharedInventories {
    channels: std::collections::BTreeMap<String, Inventory>,
}
impl SharedInventories {
    pub fn bind(&mut self, channel: String, initial: &Inventory) -> Result<Inventory, String> {
        if let Some(existing) = self.channels.get(&channel) {
            if existing.len() != initial.len() {
                return Err("shared inventory slot count differs from its channel".into());
            }
            return Ok(existing.clone());
        }
        // Moving endpoints to a new channel must never clone the old contents.
        let cells = if initial.channel().is_none() {
            initial.read().to_vec()
        } else {
            vec![None; initial.len()]
        };
        let inventory = Inventory::shared(channel.clone(), cells);
        self.channels.insert(channel, inventory.clone());
        Ok(inventory)
    }
    pub fn snapshot(&self) -> horizon_world::nbt::Tag {
        use horizon_world::nbt::Tag;
        Tag::List(
            self.channels
                .iter()
                .map(|(channel, inventory)| {
                    let cells = inventory.read();
                    Tag::compound([
                        ("channel", Tag::String(channel.as_str().into())),
                        ("slots", Tag::Int(cells.len() as i32)),
                        ("Items", crate::save::encode_items(&cells)),
                    ])
                })
                .collect(),
        )
    }
    pub fn restore(tag: &horizon_world::nbt::Tag) -> Result<Self, String> {
        use horizon_world::nbt::Tag;
        let Tag::List(entries) = tag else {
            return Err("shared inventory channels must be a list".into());
        };
        let mut result = Self::default();
        for entry in entries {
            let Some(Tag::String(channel)) = entry.get("channel") else {
                return Err("shared inventory channel missing".into());
            };
            let Some(Tag::Int(size)) = entry.get("slots") else {
                return Err("shared inventory size missing".into());
            };
            let channel = channel.to_string().map_err(|e| e.to_string())?;
            if !(0..=1024).contains(size) || channel.is_empty() || channel.len() > 321 {
                return Err("invalid shared inventory channel or size".into());
            }
            let cells = crate::save::decode_items(entry.get("Items"), *size as usize);
            if result
                .channels
                .insert(channel.clone(), Inventory::shared(channel.clone(), cells))
                .is_some()
            {
                return Err("duplicate shared inventory channel".into());
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoints_operate_on_one_inventory_even_after_all_unload() {
        let channel = Inventory::shared(
            "owner:frequency".into(),
            vec![Some(ItemStack::new("minecraft:diamond", 1))],
        );
        let a = channel.clone();
        let b = channel.clone();
        assert!(a.shares_storage_with(&b));
        let same_label_other_world = Inventory::shared("owner:frequency".into(), vec![None]);
        assert!(!a.shares_storage_with(&same_label_other_world));
        assert_eq!(a.read()[0].take().unwrap().count, 1);
        assert!(
            b.read()[0].take().is_none(),
            "the second endpoint cannot extract the last item twice"
        );
        b.read()[0] = Some(ItemStack::new("minecraft:iron_ingot", 4));
        a.read()[0].as_mut().unwrap().count -= 2;
        assert_eq!(b.read()[0].as_ref().unwrap().count, 2);
        drop((a, b));
        let reloaded = channel.clone();
        assert_eq!(reloaded.read()[0].as_ref().unwrap().count, 2);
    }
    #[test]
    fn channel_switch_and_world_save_do_not_copy_or_drop_contents() {
        let mut channels = SharedInventories::default();
        let empty: Inventory = vec![None; 27].into();
        let first = channels.bind("mod:public".into(), &empty).unwrap();
        first.read()[0] = Some(ItemStack::new("minecraft:diamond", 12));
        let second = channels.bind("mod:public".into(), &empty).unwrap();
        second.read()[0].as_mut().unwrap().count -= 5;
        let private = channels.bind("mod:private:Alice".into(), &second).unwrap();
        assert!(
            private.read()[0].is_none(),
            "switching channels must not clone seven diamonds"
        );
        private.read()[1] = Some(ItemStack::new("minecraft:gold_ingot", 3));
        let other_mod = channels.bind("other_mod:public".into(), &empty).unwrap();
        assert!(other_mod.read()[0].is_none());
        assert!(
            channels
                .bind("mod:public".into(), &vec![None; 1].into())
                .is_err()
        );
        drop((first, second, private, other_mod));
        // No endpoint is loaded when the canonical table is saved.
        let bytes =
            horizon_world::nbt::write_network(&channels.snapshot(), Default::default()).unwrap();
        let (tag, _) = horizon_world::nbt::read_network(&bytes, Default::default()).unwrap();
        let mut loaded = SharedInventories::restore(&tag).unwrap();
        let public = loaded.bind("mod:public".into(), &empty).unwrap();
        let private = loaded.bind("mod:private:Alice".into(), &empty).unwrap();
        assert_eq!(public.read()[0].as_ref().unwrap().count, 7);
        assert_eq!(private.read()[1].as_ref().unwrap().count, 3);
    }

    #[test]
    fn block_endpoints_share_slots_but_never_save_or_drop_channel_contents() {
        use crate::containers::BlockEntity;
        let mut channels = SharedInventories::default();
        let inventory = channels
            .bind(
                "mod:public".into(),
                &vec![Some(ItemStack::new("minecraft:diamond", 1))].into(),
            )
            .unwrap();
        let make = || BlockEntity::Mod {
            block: "mod:chest".into(),
            handle: 1,
            saved: vec![42],
            items: inventory.clone(),
            energy: None,
            fluids: Default::default(),
        };
        let mut a = make();
        let b = make();
        assert_eq!(a.slots_mut().slot(0).unwrap().take().unwrap().count, 1);
        assert!(b.slots()[0].is_none());
        a.slots_mut()[0] = Some(ItemStack::new("minecraft:diamond", 9));
        assert_eq!(b.slots()[0].as_ref().unwrap().count, 9);
        assert!(
            a.contents().is_empty(),
            "breaking an endpoint must not drop shared contents"
        );
        let observable = crate::save::encode_block_entity((0, 64, 0), &a, "mod:chest");
        assert!(
            observable.get("Items").is_some(),
            "commands and network snapshots must see the real slots"
        );
        let tag = crate::save::encode_block_entity_for_save((0, 64, 0), &a, "mod:chest");
        assert!(
            tag.get("Items").is_none(),
            "items belong only in the canonical world save"
        );
        let (_, restored) = crate::save::decode_block_entity(&tag).unwrap();
        assert!(
            restored.slots()[0].is_none(),
            "endpoints contain metadata, not duplicate inventories"
        );
    }

    #[test]
    fn ordinary_mod_inventories_clone_by_value() {
        let local: Inventory = vec![Some(ItemStack::new("minecraft:diamond", 1))].into();
        let copy = local.clone();
        copy.read()[0] = None;
        assert!(local.read()[0].is_some());
    }
}
