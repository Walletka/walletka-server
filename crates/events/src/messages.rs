use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightningNodeEvent {
    /// A sent payment was successful.
    PaymentSuccessful {
        /// The hash of the payment.
        payment_hash: String,
    },
    /// A sent payment has failed.
    PaymentFailed {
        /// The hash of the payment.
        payment_hash: String,
    },
    /// A payment has been received.
    PaymentReceived {
        /// The hash of the payment.
        payment_hash: String,
        /// The value, in thousandths of a satoshi, that has been received.
        amount_msat: u64,
    },
    /// A channel has been created and is pending confirmation on-chain.
    ChannelPending {
        /// The `channel_id` of the channel.
        channel_id: String,
        /// The `user_channel_id` of the channel.
        user_channel_id: String,
        /// The `temporary_channel_id` this channel used to be known by during channel establishment.
        former_temporary_channel_id: String,
        /// The `node_id` of the channel counterparty.
        counterparty_node_id: String,
        /// The outpoint of the channel's funding transaction.
        funding_txo: String,
    },
    /// A channel is ready to be used.
    ChannelReady {
        /// The `channel_id` of the channel.
        channel_id: String,
        /// The `user_channel_id` of the channel.
        user_channel_id: String,
        /// The `node_id` of the channel counterparty.
        ///
        /// This will be `None` for events serialized by LDK Node XXX TODO and prior.
        counterparty_node_id: Option<String>,
    },
    /// A channel has been closed.
    ChannelClosed {
        /// The `channel_id` of the channel.
        channel_id: String,
        /// The `user_channel_id` of the channel.
        user_channel_id: String,
        /// The `node_id` of the channel counterparty.
        ///
        /// This will be `None` for events serialized by LDK Node XXX TODO and prior.
        counterparty_node_id: Option<String>,
    },
}
