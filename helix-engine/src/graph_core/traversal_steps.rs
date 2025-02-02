use heed3::{RoTxn, RwTxn};
use protocol::{traversal_value::TraversalValue, value::Value, Edge, Node};

use crate::types::GraphError;

use super::traversal::TransactionCommit;

pub trait RSourceTraversalSteps {
    /// Adds all nodes in the graph to current traversal step
    ///
    /// Note: This can be a VERY expensive operation
    fn v(&mut self, txn: &RoTxn) -> &mut Self;
    /// Adds all edges in the graph to current traversal step
    ///  
    /// Note: This can be a VERY expensive operation
    fn e(&mut self, txn: &RoTxn) -> &mut Self;

    /// Adds node with specific id to current traversal step
    fn v_from_id(&mut self, txn: &RoTxn, node_id: &str) -> &mut Self;

    fn v_from_ids(&mut self, txn: &RoTxn, node_ids: &[String]) -> &mut Self;
    /// Adds edge with specific id to current traversal step
    fn e_from_id(&mut self, txn: &RoTxn, edge_id: &str) -> &mut Self;

    fn v_from_types(&mut self, txn: &RoTxn, node_labels: &[String]) -> &mut Self;

    // fn e_from_type(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self;

    fn v_from_secondary_index(
        &mut self,
        txn: &RoTxn,
        index: &str,
        value: &Value,
    ) -> &mut Self;
}

pub trait WSourceTraversalSteps {
    /// Creates a new node in the graph and adds it to current traversal step
    fn add_v(
        &mut self,
        txn: &mut RwTxn,
        node_label: &str,
        props: Vec<(String, Value)>,
        secondary_indices: Option<&[String]>,
    ) -> &mut Self;
    /// Creates a new edge in the graph between two nodes and adds it to current traversal step
    fn add_e(
        &mut self,
        txn: &mut RwTxn,
        edge_label: &str,
        from_id: &str,
        to_id: &str,
        props: Vec<(String, Value)>,
    ) -> &mut Self;
}

pub trait RTraversalSteps {
    /// Adds the nodes at the end of an outgoing edge to the current traversal step that match a given edge label if given one
    fn out(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self;
    /// Adds the outgoing edges from the current node to the current traversal step that match a given edge label if given one
    fn out_e(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self;

    /// Adds the nodes at the start of an incoming edge to the current traversal step that match a given edge label if given one
    fn in_(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self;
    /// Adds the incoming edges from the current node
    /// to the current traversal step that match a given edge label if given one
    fn in_e(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self;

    /// Adds the node that recieves the current edge to the current traversal step
    fn in_v(&mut self, txn: &RoTxn) -> &mut Self;

    /// Adds the node that sends the current edge to the current traversal step
    /// to the current traversal step
    fn out_v(&mut self, txn: &RoTxn) -> &mut Self;

    /// Adds the nodes at the ends of both the incoming and outgoing edges from the current node to the current traversal step
    /// that match a given edge label if given one
    fn both(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self;

    /// Adds both the incoming and outgoing edges from the current node to the current traversal step
    /// that match a given edge label if given one
    fn both_e(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self;

    fn mutual(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self;

    // fn mutual_e(&mut self, txn: &RoTxn, edge_label: &str) -> &mut Self;

    /// Adds the nodes at the ends of both the incoming and outgoing edges from the current node to the current traversal step
    fn both_v(&mut self, txn: &RoTxn) -> &mut Self;
}

pub trait WTraversalSteps {
    /// Creates a new edge in the graph between two nodes and adds it to current traversal step
    fn add_e_to(
        &mut self,
        txn: &mut RwTxn,
        edge_label: &str,
        to_id: &str,
        props: Vec<(String, Value)>,
    ) -> &mut Self;

    /// Creates a new edge in the graph between two nodes and adds it to current traversal step
    fn add_e_from(
        &mut self,
        txn: &mut RwTxn,
        edge_label: &str,
        from_id: &str,
        props: Vec<(String, Value)>,
    ) -> &mut Self;

    fn update_props(&mut self, txn: &mut RwTxn, props: Vec<(String, Value)>) -> &mut Self;
}

pub trait TraversalMethods {
    /// Flattens everything in the current traversal step and counts how many items there are.
    fn count(&mut self) -> &mut Self;

    /// Flattens everything in the current traversal step and updates the current traversal step to be a slice of itself.
    fn range(&mut self, start: usize, end: usize) -> &mut Self;

    /// Filters the current traversal step
    ///
    /// ### Returns:
    /// - The traversal builder with the current step overwritten with the remaining values
    ///
    /// ## Example
    /// ```rust
    ///
    /// use helix_engine::{
    ///     graph_core::traversal_steps::{SourceTraversalSteps, TraversalMethods, TraversalSteps},
    ///     graph_core::graph_core::HelixGraphEngine,
    ///     graph_core::traversal::TraversalBuilder,
    ///     props,
    ///     storage_core::{storage_core::HelixGraphStorage, storage_methods::StorageMethods},
    ///     types::GraphError,
    ///     
    /// };
    /// use protocol::{count::Count, traversal_value::TraversalValue, Edge, filterable::Filterable, Node, value::Value};
    /// use std::collections::HashMap;
    /// use tempfile::TempDir;
    /// use std::sync::Arc;
    ///
    /// let temp_dir = TempDir::new().unwrap();
    /// let db_path = temp_dir.path().to_str().unwrap();
    /// let engine = HelixGraphEngine::new(db_path).unwrap();
    ///
    /// let _ = engine.storage
    ///     .create_node("person", props! { "age" => 25, "name" => "Alice" })
    ///     .unwrap();
    /// let person2 = engine.storage
    ///     .create_node("person", props! { "age" => 30, "name" => "Bob" })
    ///     .unwrap();
    /// let _ = engine.storage
    ///     .create_node("person", props! { "age" => 35 })
    ///     .unwrap();
    ///
    ///
    /// fn age_greater_than(val: &Node, min_age: i32) -> Result<bool, GraphError> {
    ///     if let Some(value) = val.check_property("age") {
    ///         match value {
    ///             Value::Float(age) => Ok(*age > min_age as f64),
    ///             Value::Integer(age) => Ok(*age > min_age),
    ///             _ => Err(GraphError::TraversalError("Invalid type".to_string())),
    ///         }
    ///     } else {
    ///         Err(GraphError::TraversalError("Invalid node".to_string()))
    ///     }
    /// }
    ///
    /// fn has_name(val: &Node) -> Result<bool, GraphError> {
    ///     return Ok(val.check_property("name").is_some());
    /// }
    ///
    /// // Example With Closure
    /// let mut traversal = TraversalBuilder::new(Arc::clone(&engine.storage), TraversalValue::Empty);
    /// let test_with_closure = traversal.v().filter_nodes(|val| {
    ///     if let Some(value) = val.check_property("age") {
    ///         match value {
    ///             Value::Float(age) => Ok(*age > 25.0),
    ///             Value::Integer(age) => Ok(*age > 25),
    ///             _ => Err(GraphError::TraversalError("Invalid type".to_string())),
    ///         }
    ///     } else {
    ///         Err(GraphError::TraversalError("No age property".to_string()))
    ///     }
    /// }).count();
    /// if let TraversalValue::Count(count) = &test_with_closure.current_step {
    ///     assert_eq!(count.value(), 2, "Closure");
    /// } else {
    ///     panic!("Expected Count value");
    /// }
    ///    
    /// // Example passing function that takes input
    /// let mut traversal = TraversalBuilder::new(Arc::clone(&engine.storage), TraversalValue::Empty);
    /// let test_calling_function_with_inputs = traversal.v().filter_nodes(|node| age_greater_than(node, 30)).count();
    /// if let TraversalValue::Count(count) = &test_calling_function_with_inputs.current_step {
    ///     assert_eq!(count.value(), 1, "W input");
    /// } else {
    ///     panic!("Expected Count value");
    /// }
    ///  
    /// // Example passing function that takes NO input
    /// let mut traversal = TraversalBuilder::new(Arc::clone(&engine.storage), TraversalValue::Empty);
    /// let test_calling_function_without_inputs = traversal.v().filter_nodes(has_name).count();
    /// if let TraversalValue::Count(count) = &test_calling_function_without_inputs.current_step {
    ///     assert_eq!(count.value(), 2, "WO input");
    /// } else {
    ///     panic!("Expected Count value");
    /// }
    ///
    ///
    /// // Example of chained traversal
    /// let mut traversal = TraversalBuilder::new(Arc::clone(&engine.storage), TraversalValue::Empty);
    /// let test_chained_traversal = traversal.v()
    ///     .filter_nodes(has_name)
    ///     .filter_nodes(|val| age_greater_than(val, 27)).count();
    /// if let TraversalValue::Count(count) = &test_chained_traversal.current_step {
    ///     assert_eq!(count.value(), 1, "Chained");
    /// } else {
    ///     panic!("Expected Count value");
    /// }
    ///
    ///
    ///
    /// ```
    fn filter_nodes<F>(&mut self, txn: &RoTxn, predicate: F) -> &mut Self
    where
        F: Fn(&Node) -> Result<bool, GraphError>;

    fn filter_edges<F>(&mut self, txn: &RoTxn, predicate: F) -> &mut Self
    where
        F: Fn(&Edge) -> Result<bool, GraphError>;

    /// Maps the current traversal step to a new traversal step
    fn get_properties(&mut self, txn: &RoTxn, keys: &Vec<String>) -> &mut Self;

    /// Maps the current traversal step to a new traversal step
    fn map_nodes<F>(&mut self, txn: &RoTxn, map_fn: F) -> &mut Self
    where
        F: Fn(&Node) -> Result<Node, GraphError>;

    /// Maps the current traversal step to a new traversal step
    fn map_edges<F>(&mut self, txn: &RoTxn, map_fn: F) -> &mut Self
    where
        F: Fn(&Edge) -> Result<Edge, GraphError>;

    fn for_each_node<F>(&mut self, txn: &RoTxn, map_fn: F) -> &mut Self
    where
        F: Fn(&Node, &RoTxn) -> Result<(), GraphError>;

    fn for_each_node_mut<F>(&mut self, txn: &mut RwTxn, map_fn: F) -> &mut Self
    where
        F: Fn(&Node, &mut RwTxn) -> Result<(), GraphError>;

    fn for_each_edge<F>(&mut self, txn: &RoTxn, map_fn: F) -> &mut Self
    where
        F: Fn(&Edge) -> Result<(), GraphError>;
}

pub trait RTraversalBuilderMethods {
    //// / Finishes the result and returns the final current traversal step
    fn result<'a, T>(self, txn: &mut T) -> Result<TraversalValue, GraphError>
    where
        T: AsMut<RoTxn<'a>>;

    fn execute(self) -> Result<(), GraphError>;
}

pub trait WTraversalBuilderMethods {
    /// Finishes the result and returns the final current traversal step
    fn result<T>(self, txn: T) -> Result<TraversalValue, GraphError>
    where
        T: TransactionCommit;

    fn execute(self) -> Result<(), GraphError>;
}

pub trait TraversalSearchMethods {
    /// Finds the shortest path from a given node to the currnet node using BFS
    fn shortest_path_from(&mut self, txn: &RoTxn, from_id: &str) -> &mut Self;

    /// Finds the shortes path from the current node to a given node using BFS
    fn shortest_path_to(&mut self, txn: &RoTxn, to_id: &str) -> &mut Self;

    /// Finds the shortes path between two given nodes using BFS
    fn shortest_path_between(&mut self, txn: &RoTxn, from_id: &str, to_id: &str) -> &mut Self;
}
