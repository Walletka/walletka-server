use log::info;
use nostr_sdk::prelude::*;

pub struct NostrClient {
    _keys: Keys,
    client: Client,
}

impl NostrClient {
    pub fn new() -> Self {
        let keys = Keys::generate();
        let client = Client::new(&keys);

        info!(
            "Nostr generated\nnpub: {:?}\nnsec: {:?}",
            keys.public_key().to_bech32().unwrap(),
            keys.secret_key().unwrap().display_secret().to_string()
        );

        Self {
            _keys: keys,
            client,
        }
    }

    pub fn from_mnemonic(mnemonic: &str, passphrase: Option<&str>) -> Self {
        let keys = Keys::from_mnemonic(mnemonic, passphrase).unwrap();
        let client = Client::new(&keys);

        info!("Nostr keys: {:?}", keys.public_key().to_bech32().unwrap());

        Self {
            _keys: keys,
            client,
        }
    }

    pub fn from_secret_key(secret_key: &str) -> Self {
        let keys = Keys::from_sk_str(secret_key).unwrap();
        let client = Client::new(&keys);

        info!("Nostr keys: {:?}", keys.public_key().to_bech32().unwrap());

        Self {
            _keys: keys,
            client,
        }
    }

    pub async fn start(&self, default_relay: &str) -> Result<(), anyhow::Error> {
        self.client.add_relay(default_relay).await?;

        self.client.connect().await;

        Ok(())
    }

    pub async fn send_message(&self, recipient: String, msg: &str) -> Result<(), anyhow::Error> {
        self.client
            .send_direct_msg(XOnlyPublicKey::from_bech32(recipient)?, msg, None)
            .await?;

        Ok(())
    }
}
