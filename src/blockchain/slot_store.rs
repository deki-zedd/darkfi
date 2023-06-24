/* This file is part of DarkFi (https://dark.fi)
 *
 * Copyright (C) 2020-2023 Dyne.org foundation
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use darkfi_sdk::blockchain::Slot;
use darkfi_serial::{deserialize, serialize};

use crate::{blockchain::SledDbOverlayPtr, Error, Result};

const SLED_SLOT_TREE: &[u8] = b"_slots";

/// The `SlotStore` is a `sled` tree storing the blockhains' slots,
/// where the key is the slot uid, and the value is is the serialized slot.
#[derive(Clone)]
pub struct SlotStore(sled::Tree);

impl SlotStore {
    /// Opens a new or existing `SlotStore` on the given sled database.
    pub fn new(db: &sled::Db, genesis_block: blake3::Hash) -> Result<Self> {
        let tree = db.open_tree(SLED_SLOT_TREE)?;
        let store = Self(tree);

        // In case the store is empty, initialize it with the genesis slot.
        if store.0.is_empty() {
            let genesis_slot = Slot::genesis_slot(genesis_block);
            store.insert(&[genesis_slot])?;
        }

        Ok(store)
    }

    /// Insert a slice of [`Slot`] into the slot store.
    /// With sled, the operation is done as a batch.
    /// The slot id is used as the key, while value is the serialized [`Slot`] itself.
    pub fn insert(&self, slots: &[Slot]) -> Result<()> {
        let mut batch = sled::Batch::default();

        for slot in slots {
            let serialized = serialize(slot);
            batch.insert(&slot.id.to_be_bytes(), serialized);
        }

        self.0.apply_batch(batch)?;
        Ok(())
    }

    /// Check if the slot store contains a given id.
    pub fn contains(&self, id: u64) -> Result<bool> {
        Ok(self.0.contains_key(id.to_be_bytes())?)
    }

    /// Fetch given slots from the slot store.
    /// The resulting vector contains `Option`, which is `Some` if the slot
    /// was found in the slot store, and otherwise it is `None`, if it has not.
    /// The second parameter is a boolean which tells the function to fail in
    /// case at least one slot was not found.
    pub fn get(&self, ids: &[u64], strict: bool) -> Result<Vec<Option<Slot>>> {
        let mut ret = Vec::with_capacity(ids.len());

        for id in ids {
            if let Some(found) = self.0.get(id.to_be_bytes())? {
                let slot = deserialize(&found)?;
                ret.push(Some(slot));
            } else {
                if strict {
                    return Err(Error::SlotNotFound(*id))
                }
                ret.push(None);
            }
        }

        Ok(ret)
    }

    /// Retrieve all slot from the slot store.
    /// Be careful as this will try to load everything in memory.
    pub fn get_all(&self) -> Result<Vec<Slot>> {
        let mut slots = vec![];

        for slot in self.0.iter() {
            let (_, value) = slot.unwrap();
            let slot = deserialize(&value)?;
            slots.push(slot);
        }

        Ok(slots)
    }

    /// Fetch n slots after given slot. In the iteration, if a slot is not
    /// found, the iteration stops and the function returns what it has found
    /// so far in the `SlotStore`.
    pub fn get_after(&self, id: u64, n: u64) -> Result<Vec<Slot>> {
        let mut ret = vec![];

        let mut key = id;
        let mut counter = 0;
        while counter <= n {
            if let Some(found) = self.0.get_gt(key.to_be_bytes())? {
                let key_bytes: [u8; 8] = found.0.as_ref().try_into().unwrap();
                key = u64::from_be_bytes(key_bytes);
                let slot = deserialize(&found.1)?;
                ret.push(slot);
                counter += 1;
                continue
            }
            break
        }

        Ok(ret)
    }

    /// Fetch the last slot in the tree, based on the `Ord`
    /// implementation for `Vec<u8>`. This should not be able to
    /// fail because we initialize the store with the genesis slot.
    pub fn get_last(&self) -> Result<Slot> {
        let found = self.0.last()?.unwrap();
        let slot = deserialize(&found.1)?;
        Ok(slot)
    }

    /// Retrieve records count
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }
}

/// Overlay structure over a [`SlotStore`] instance.
pub struct SlotStoreOverlay(SledDbOverlayPtr);

impl SlotStoreOverlay {
    pub fn new(overlay: SledDbOverlayPtr) -> Result<Self> {
        overlay.lock().unwrap().open_tree(SLED_SLOT_TREE)?;
        Ok(Self(overlay))
    }

    /// Fetch given id from the slot store.
    pub fn get(&self, id: u64) -> Result<Vec<u8>> {
        match self.0.lock().unwrap().get(SLED_SLOT_TREE, &id.to_be_bytes())? {
            Some(found) => Ok(found.to_vec()),
            None => Err(Error::SlotNotFound(id)),
        }
    }
}
