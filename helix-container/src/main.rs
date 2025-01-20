extern crate graph_queries;
use chrono::Utc;
use helix_engine::{
    graph_core::graph_core::HelixGraphEngine,
    props,
    storage_core::{storage_core::HelixGraphStorage, storage_methods::StorageMethods},
};
use helix_gateway::{
    router::router::{HandlerFn, HandlerSubmission},
    GatewayOpts, HelixGateway,
};
use inventory;
use rand::Rng;
use std::{collections::HashMap, sync::Arc};

mod traversals;
use traversals::*;  

fn main() {
    let path = "~/.helix/user";
    let graph = Arc::new(HelixGraphEngine::new(path).unwrap());
    create_test_graph(Arc::clone(&graph), 100, 10);

    // generates routes from handler proc macro
    println!("Starting route collection...");
    let submissions: Vec<_> = inventory::iter::<HandlerSubmission>.into_iter().collect();
    println!("Found {} submissions", submissions.len());

    let routes = HashMap::from_iter(
        submissions
            .into_iter()
            .map(|submission| {
                println!("Processing submission for handler: {}", submission.0.name);
                let handler = &submission.0;
                let func: HandlerFn =
                    Arc::new(move |input, response| (handler.func)(input, response));
                (
                    (
                        "post".to_ascii_uppercase().to_string(),
                        format!("/{}", handler.name.to_string()),
                    ),
                    func,
                )
            })
            .collect::<Vec<((String, String), HandlerFn)>>(),
    );


    println!("Routes: {:?}", routes.keys());
    // create gateway
    let gateway = HelixGateway::new(
        "127.0.0.1:3001",
        graph,
        GatewayOpts::DEFAULT_POOL_SIZE,
        Some(routes),
    );

    // start server
    let _ = gateway.connection_handler.accept_conns().join().unwrap(); // TODO handle error causes panic
}

fn create_test_graph(graph: Arc<HelixGraphEngine>, size: usize, edges_per_node: usize) {
    let storage = &graph.storage; //.lock().unwrap();
    let mut node_ids = Vec::with_capacity(size);

    for _ in 0..size {
        let node = storage.create_node("person", props!()).unwrap();
        node_ids.push(node.id);
    }

    let mut rng = rand::thread_rng();
    for from_id in &node_ids {
        for _ in 0..edges_per_node {
            let to_index = rng.gen_range(0..size);
            let to_id = &node_ids[to_index];

            if from_id != to_id {
                storage
                    .create_edge("knows", from_id, to_id, props!())
                    .unwrap();
            }
        }
    }
}
