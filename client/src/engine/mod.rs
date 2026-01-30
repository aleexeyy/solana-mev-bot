use std::sync::Arc;

use tokio::sync::mpsc::Receiver;

use crate::{graph::Graph, shred_decoders::interfaces::ShredEvent};

pub mod pools;

pub async fn receive_decoded_transactions(
    mut simulate_rx: Receiver<Vec<ShredEvent>>,
    _graph: Arc<Graph>,
) {
    while let Some(_events) = simulate_rx.recv().await {
        // tracing::warn!("receiving shred events: {:?}", _events);
    }

    // TODO: receive transactions, add the method to the graph called simulate_transaction -> simulate_instruction, and we simulate it one by one, on the arbitrage routes and then look at them and check wether any is profitable
}
