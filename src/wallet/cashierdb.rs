use super::WalletApi;
use crate::client::ClientFailed;
use crate::service::btc::{PrivKey, PubKey};
use crate::{Error, Result};

use async_std::sync::Arc;

use bitcoin::Address as BtcAddr;
use log::*;
use rusqlite::{named_params, Connection, params};

use std::path::PathBuf;
use std::str::FromStr;

pub type CashierDbPtr = Arc<CashierDb>;

pub struct CashierDb {
    pub path: PathBuf,
    pub password: String,
}

impl WalletApi for CashierDb {
    fn get_password(&self) -> String {
        self.password.to_owned()
    }
    fn get_path(&self) -> PathBuf {
        self.path.to_owned()
    }
}

impl CashierDb {
    pub fn new(path: &PathBuf, password: String) -> Result<CashierDbPtr> {
        debug!(target: "CASHIERDB", "new() Constructor called");
        Ok(Arc::new(Self {
            path: path.to_owned(),
            password,
        }))
    }

    pub fn init_db(&self) -> Result<()> {
        if !self.password.trim().is_empty() {
            let contents = include_str!("../../res/cashier.sql");
            let conn = Connection::open(&self.path)?;
            debug!(target: "CASHIERDB", "Opened connection at path {:?}", self.path);
            conn.pragma_update(None, "key", &self.password)?;
            conn.execute_batch(&contents)?;
        } else {
            debug!(target: "CASHIERDB", "Password is empty. You must set a password to use the wallet.");
            return Err(Error::from(ClientFailed::EmptyPassword));
        }
        Ok(())
    }

    pub fn get_keys_by_dkey(&self, dkey_pub: &Vec<u8>) -> Result<()> {
        debug!(target: "CASHIERDB", "Check for existing dkey");
        //let dkey_id = self.get_value_deserialized(dkey_pub)?;
        // open connection
        let conn = Connection::open(&self.path)?;
        // unlock database
        conn.pragma_update(None, "key", &self.password)?;

        // let mut keypairs = conn.prepare("SELECT dkey_id FROM keypairs WHERE dkey_id = :dkey_id")?;
        // let rows = keypairs.query_map::<Vec<u8>, _, _>(&[(":dkey_id", &secret)], |row| row.get(0))?;

        let mut stmt = conn.prepare("SELECT * FROM keypairs where dkey_id = ?")?;
        let mut rows = stmt.query([dkey_pub])?;
        if let Some(_row) = rows.next()? {
            println!("Got something");
        } else {
            println!("Did not get something");
        }

        Ok(())
    }

    // Update to take BitcoinKeys instance instead
    pub fn put_exchange_keys(
        &self,
        dkey_pub: Vec<u8>,
        btc_private: PrivKey,
        btc_public: PubKey,
        //txid will be updated when exists
    ) -> Result<()> {
        debug!(target: "CASHIERDB", "Put exchange keys");
        // prepare the values
        //let dkey_pub = self.get_value_serialized(&dkey_pub)?;
        let btc_private = btc_private.to_bytes();
        let btc_public = btc_public.to_bytes();

        // open connection
        let conn = Connection::open(&self.path)?;
        // unlock database
        conn.pragma_update(None, "key", &self.password)?;

        conn.execute(
            "INSERT INTO keypairs(dkey_id, btc_key_private, btc_key_public)
            VALUES (:dkey_id, :btc_key_private, :btc_key_public)",
            named_params! {
                ":dkey_id": dkey_pub,
                ":btc_key_private": btc_private,
                ":btc_key_private": btc_public,
            },
        )?;
        Ok(())
    }

    pub fn get_withdraw_private_keys(&self) -> Result<Vec<jubjub::Fr>> {
        debug!(target: "CASHIERDB", "Get withdraw private keys");
        // open connection
        let conn = Connection::open(&self.path)?;
        // unlock database
        conn.pragma_update(None, "key", &self.password)?;

        let mut stmt = conn.prepare("SELECT d_key_private FROM withdraw_keypairs")?;
        let keys = stmt.query_map([], |row| {
            let private_key: jubjub::Fr = self
                .get_value_deserialized(row.get(0)?)
                .expect("deserialize private key");
            Ok(private_key)
        })?;

        let mut private_keys: Vec<jubjub::Fr> = vec![];

        for k in keys {
            private_keys.push(k?);
        }

        Ok(private_keys)
    }

    // return (public key, private key)
    pub fn get_address_by_btc_key(
        &self,
        btc_address: &Vec<u8>,
    ) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
        debug!(target: "CASHIERDB", "Check for existing btc address");
        // open connection
        let conn = Connection::open(&self.path)?;
        // unlock database
        conn.pragma_update(None, "key", &self.password)?;

        let mut stmt =
            conn.prepare("SELECT * FROM withdraw_keypairs where btc_key_id = :btc_key_id")?;
        let addr_iter = stmt
            .query_map::<(Vec<u8>, Vec<u8>), _, _>(&[(":btc_key_id", btc_address)], |row| {
                Ok((row.get(2)?, row.get(1)?))
            })?;

        let mut btc_addresses = vec![];

        for addr in addr_iter {
            btc_addresses.push(addr);
        }

        if let Some(addr) = btc_addresses.pop() {
            return Ok(Some(addr?));
        }

        return Ok(None);
    }

    pub fn get_btc_addr_by_address(
        &self,
        pub_key: jubjub::SubgroupPoint,
    ) -> Result<Option<BtcAddr>> {
        debug!(target: "CASHIERDB", "Get btc address by pub_key");
        // open connection
        let conn = Connection::open(&self.path)?;
        // unlock database
        conn.pragma_update(None, "key", &self.password)?;

        let d_key_public = self.get_value_serialized(&pub_key)?;

        let mut stmt = conn.prepare(
            "SELECT btc_key_id FROM withdraw_keypairs where d_key_public = :d_key_public",
        )?;
        let addr_iter =
            stmt.query_map::<BtcAddr, _, _>(&[(":d_key_public", &d_key_public)], |row| {
                let btc_address: String = self
                    .get_value_deserialized(row.get(0)?)
                    .expect("deserialize btc address");
                let btc_address = bitcoin::util::address::Address::from_str(&btc_address)
                    .expect("get btc_address from_str");
                Ok(btc_address)
            })?;

        let mut btc_addresses: Vec<BtcAddr> = vec![];

        for addr in addr_iter {
            btc_addresses.push(addr?);
        }

        Ok(btc_addresses.pop())
    }

    pub fn delete_withdraw_key_record(
        &self,
        btc_address: BtcAddr,
    ) -> Result<()> {
        debug!(target: "CASHIERDB", "Delete withdraw keys");

        // open connection
        let conn = Connection::open(&self.path)?;
        // unlock database
        conn.pragma_update(None, "key", &self.password)?;

        let btc_address = btc_address.to_string();
        let btc_address = self.get_value_serialized(&btc_address)?;

        conn.execute(
            "DELETE FROM withdraw_keypairs WHERE btc_key_id = ?1;",
            params! [
                btc_address
            ],
        )?;

        Ok(())
    }

    pub fn put_withdraw_keys(
        &self,
        btc_key_id: Vec<u8>,
        d_key_public: Vec<u8>,
        d_key_private: Vec<u8>,
    ) -> Result<()> {
        debug!(target: "CASHIERDB", "Put withdraw keys");

        // open connection
        let conn = Connection::open(&self.path)?;
        // unlock database
        conn.pragma_update(None, "key", &self.password)?;

        conn.execute(
            "INSERT INTO withdraw_keypairs(btc_key_id, d_key_private, d_key_public)
            VALUES (:btc_key_id, :d_key_private, :d_key_public)",
            named_params! {
                ":btc_key_id": btc_key_id,
                ":d_key_private": d_key_private,
                ":d_key_public": d_key_public,
            },
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::serial::serialize;
    use crate::util::join_config_path;

    use crate::serial;
    use ff::Field;
    use rand::rngs::OsRng;

    #[test]
    pub fn test_put_withdraw_keys_and_load_them_with_btc_key() -> Result<()> {
        let walletdb_path = join_config_path(&PathBuf::from("cashier_wallet_test.db"))?;
        let wallet = CashierDb::new(&walletdb_path, "darkfi".into())?;
        wallet.init_db()?;

        let secret2: jubjub::Fr = jubjub::Fr::random(&mut OsRng);
        let public2 = zcash_primitives::constants::SPENDING_KEY_GENERATOR * secret2;
        let key_public2 = serial::serialize(&public2);
        let key_private2 = serial::serialize(&secret2);

        let btc_addr = serialize(&String::from("bc10000000000000000000000000000000000000000"));

        wallet.put_withdraw_keys(btc_addr.clone(), key_public2.clone(), key_private2.clone())?;

        let addr = wallet.get_address_by_btc_key(&btc_addr)?;

        assert_eq!(addr, Some((key_public2, key_private2)));

        wallet.destroy()?;

        Ok(())
    }
}
