#[tokio::test]
async fn test_graph_and_cycles_setup() {
    let test_folder: &str = "./tests/test_data";
    let test_depth: usize = 4;

    //for the first run without test folder
    // let _ = solana_mev_bot::bootstrap::update_all(test_folder, true).await.unwrap();
    let mut graph = client::graph::Graph::build_graph(test_folder).unwrap();

    assert_eq!(graph.edges.len(), 138);
    assert_eq!(graph.nodes.len(), 105);

    graph.build_cycles(test_depth).unwrap();

    assert_eq!(graph.all_cycles.len(), 61);

    let mut invalid_cycle_counter: usize = 0;
    for cycles in graph.all_cycles.clone() {
        for mut cycle in cycles.1 {
            assert!(cycle.len() <= test_depth);
            if graph.check_cycle(cycle.as_mut()) {
                invalid_cycle_counter += 1;
            }
        }
    }

    assert_eq!(invalid_cycle_counter, 0);
}
