//! Guest-owned machine state with a cached selection of received handles.
//!
//! Only selected machines are visited, once each, in storage order. This does
//! not promise the input order. Indices are rebuilt when handles or membership
//! change; neither references nor pointers into the storage are retained.
use std::{collections::HashMap, ops::Index};

pub struct ActiveMachines<T> {
    indices: HashMap<u32, usize>,
    slots: Vec<Option<(u32, T)>>,
    free: Vec<usize>,
    selected_handles: Vec<u32>,
    selected_indices: Vec<usize>,
    all_dense: bool,
}

impl<T> Default for ActiveMachines<T> {
    fn default() -> Self {
        Self {
            indices: HashMap::new(),
            slots: Vec::new(),
            free: Vec::new(),
            selected_handles: Vec::new(),
            selected_indices: Vec::new(),
            all_dense: false,
        }
    }
}

impl<T> ActiveMachines<T> {
    fn invalidate(&mut self) {
        self.selected_handles.clear();
        self.selected_indices.clear();
        self.all_dense = false;
    }
    pub fn insert(&mut self, handle: u32, value: T) -> Option<T> {
        self.invalidate();
        if let Some(&index) = self.indices.get(&handle) {
            return self.slots[index]
                .replace((handle, value))
                .map(|(_, old)| old);
        }
        let index = self.free.pop().unwrap_or_else(|| {
            self.slots.push(None);
            self.slots.len() - 1
        });
        self.slots[index] = Some((handle, value));
        self.indices.insert(handle, index);
        None
    }
    pub fn remove(&mut self, handle: &u32) -> Option<T> {
        let index = self.indices.remove(handle)?;
        self.invalidate();
        let (_, value) = self.slots[index].take()?;
        self.free.push(index);
        Some(value)
    }
    pub fn get(&self, handle: &u32) -> Option<&T> {
        self.slots[*self.indices.get(handle)?]
            .as_ref()
            .map(|(_, value)| value)
    }
    pub fn get_mut(&mut self, handle: &u32) -> Option<&mut T> {
        self.slots[*self.indices.get(handle)?]
            .as_mut()
            .map(|(_, value)| value)
    }
    pub fn contains_key(&self, handle: &u32) -> bool {
        self.indices.contains_key(handle)
    }
    pub fn len(&self) -> usize {
        self.indices.len()
    }
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.slots.iter().flatten().map(|(_, value)| value)
    }
    pub fn for_each_active(&mut self, handles: &[u32], mut update: impl FnMut(&mut T)) {
        if self.selected_handles.as_slice() != handles {
            self.selected_handles.clear();
            self.selected_handles.extend_from_slice(handles);
            self.selected_indices.clear();
            self.selected_indices.extend(
                handles
                    .iter()
                    .filter_map(|handle| self.indices.get(handle).copied()),
            );
            self.selected_indices.sort_unstable();
            self.selected_indices.dedup();
            self.all_dense = self.selected_indices.len() == self.indices.len()
                && self.slots.len() == self.indices.len();
        }
        if self.all_dense {
            for (_, value) in self.slots.iter_mut().flatten() {
                update(value);
            }
        } else {
            for &index in &self.selected_indices {
                if let Some((_, value)) = self.slots[index].as_mut() {
                    update(value);
                }
            }
        }
    }
}
impl<T> Index<&u32> for ActiveMachines<T> {
    type Output = T;
    fn index(&self, handle: &u32) -> &Self::Output {
        self.get(handle).expect("unknown machine handle")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reordered_subsets_unknowns_and_duplicates_visit_only_live_selected_state() {
        let mut machines = ActiveMachines::default();
        for handle in [7, 2, 90] {
            machines.insert(handle, 0);
        }
        machines.for_each_active(&[90, 7, 90, 999], |value| *value += 1);
        assert_eq!((machines[&7], machines[&2], machines[&90]), (1, 0, 1));
        machines.for_each_active(&[90, 7, 90, 999], |value| *value += 1);
        assert_eq!((machines[&7], machines[&2], machines[&90]), (2, 0, 2));
        machines.for_each_active(&[2], |value| *value += 1);
        machines.for_each_active(&[], |value| *value += 1);
        assert_eq!((machines[&7], machines[&2], machines[&90]), (2, 1, 2));
    }
    #[test]
    fn removal_and_slot_reuse_invalidate_a_previously_cached_selection() {
        let mut machines = ActiveMachines::default();
        machines.insert(7, 0);
        machines.insert(2, 0);
        machines.for_each_active(&[7, 2], |value| *value += 1);
        assert_eq!(machines.remove(&7), Some(1));
        machines.insert(90, 100);
        machines.for_each_active(&[7, 2], |value| *value += 1);
        assert_eq!(
            machines[&90], 100,
            "old handle must not reach reused storage"
        );
        assert_eq!(machines[&2], 2);
        machines.for_each_active(&[90, 2], |value| *value += 1);
        assert_eq!((machines[&90], machines[&2]), (101, 3));
        for _ in 0..1000 {
            machines.remove(&90);
            machines.insert(90, 0);
        }
        assert_eq!(machines.slots.len(), 2, "unload/reload must reuse storage");
    }
    #[test]
    fn partial_unknown_selection_cannot_enable_the_all_dense_path() {
        let mut machines = ActiveMachines::default();
        machines.insert(7, 0);
        machines.insert(2, 0);
        machines.for_each_active(&[7, 999], |value| *value += 1);
        assert_eq!((machines[&7], machines[&2]), (1, 0));
        machines.for_each_active(&[7, 7], |value| *value += 1);
        assert_eq!((machines[&7], machines[&2]), (2, 0));
    }
}
