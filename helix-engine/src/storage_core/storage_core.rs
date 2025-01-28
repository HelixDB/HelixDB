use bincode::{deserialize, serialize};
use heed3::{types::*, Database, Env, EnvOpenOptions, RoTxn, RwTxn};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;
use uuid::Uuid;

use crate::storage_core::storage_methods::{SearchMethods, StorageMethods};
use crate::types::GraphError;
use protocol::{value::Value, Edge, Node};

// Database names for different stores
const DB_NODES: &str = "nodes"; // For node data (n:)
const DB_EDGES: &str = "edges"; // For edge data (e:)
const DB_NODE_LABELS: &str = "node_labels"; // For node label indices (nl:)
const DB_EDGE_LABELS: &str = "edge_labels"; // For edge label indices (el:)
const DB_OUT_EDGES: &str = "out_edges"; // For outgoing edge indices (o:)
const DB_IN_EDGES: &str = "in_edges"; // For incoming edge indices (i:)

// Key prefixes for different types of data
const NODE_PREFIX: &[u8] = b"n:";
const EDGE_PREFIX: &[u8] = b"e:";
const NODE_LABEL_PREFIX: &[u8] = b"nl:";
const EDGE_LABEL_PREFIX: &[u8] = b"el:";
const OUT_EDGES_PREFIX: &[u8] = b"o:";
const IN_EDGES_PREFIX: &[u8] = b"i:";

pub struct HelixGraphStorage {
    pub env: Env,
    nodes_db: Database<Bytes, Bytes>,
    edges_db: Database<Bytes, Bytes>,
    node_labels_db: Database<Bytes, Unit>,
    edge_labels_db: Database<Bytes, Unit>,
    out_edges_db: Database<Bytes, Unit>,
    in_edges_db: Database<Bytes, Unit>,
}

impl HelixGraphStorage {
    pub fn new(path: &str) -> Result<HelixGraphStorage, GraphError> {
        fs::create_dir_all(path)?;

        // Configure and open LMDB environment
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(10 * 1024 * 1024 * 1024) // 10GB max
                .max_dbs(6)
                .max_readers(126)
                .open(Path::new(path))?
        };

        let mut wtxn = env.write_txn()?;

        // Create/open all necessary databases
        let nodes_db = env.create_database(&mut wtxn, Some(DB_NODES))?;
        let edges_db = env.create_database(&mut wtxn, Some(DB_EDGES))?;
        let node_labels_db = env.create_database(&mut wtxn, Some(DB_NODE_LABELS))?;
        let edge_labels_db = env.create_database(&mut wtxn, Some(DB_EDGE_LABELS))?;
        let out_edges_db = env.create_database(&mut wtxn, Some(DB_OUT_EDGES))?;
        let in_edges_db = env.create_database(&mut wtxn, Some(DB_IN_EDGES))?;

        wtxn.commit()?;

        Ok(Self {
            env,
            nodes_db,
            edges_db,
            node_labels_db,
            edge_labels_db,
            out_edges_db,
            in_edges_db,
        })
    }

    #[inline(always)]
    pub fn new_node(label: &str, properties: impl IntoIterator<Item = (String, Value)>) -> Node {
        Node {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            properties: HashMap::from_iter(properties),
        }
    }

    #[inline(always)]
    pub fn new_edge(
        label: &str,
        from_node: &str,
        to_node: &str,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Edge {
        Edge {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            properties: HashMap::from_iter(properties),
        }
    }

    
    #[inline(always)]
    pub fn node_key(id: &str) -> Vec<u8> {
        [NODE_PREFIX, id.as_bytes()].concat()
    }

    #[inline(always)]
    pub fn edge_key(id: &str) -> Vec<u8> {
        [EDGE_PREFIX, id.as_bytes()].concat()
    }

    #[inline(always)]
    pub fn node_label_key(label: &str, id: &str) -> Vec<u8> {
        [NODE_LABEL_PREFIX, label.as_bytes(), b":", id.as_bytes()].concat()
    }

    #[inline(always)]
    pub fn edge_label_key(label: &str, id: &str) -> Vec<u8> {
        [EDGE_LABEL_PREFIX, label.as_bytes(), b":", id.as_bytes()].concat()
    }

    #[inline(always)]
    pub fn out_edge_key(source_node_id: &str, edge_id: &str) -> Vec<u8> {
        [
            OUT_EDGES_PREFIX,
            source_node_id.as_bytes(),
            b":",
            edge_id.as_bytes(),
        ]
        .concat()
    }

    #[inline(always)]
    pub fn in_edge_key(sink_node_id: &str, edge_id: &str) -> Vec<u8> {
        [
            IN_EDGES_PREFIX,
            sink_node_id.as_bytes(),
            b":",
            edge_id.as_bytes(),
        ]
        .concat()
    }
}

impl StorageMethods for HelixGraphStorage {
    #[inline(always)]
    fn check_exists(&self, txn: &RoTxn<'_>, id: &str) -> Result<bool, GraphError> {
        let exists = self
            .nodes_db
            .get(&txn, Self::node_key(id).as_slice())?
            .is_some();
        Ok(exists)
    }

    #[inline(always)]
    fn get_temp_node(&self, txn: &RoTxn<'_>, id: &str) -> Result<Node, GraphError> {
        match self.nodes_db.get(&txn, Self::node_key(id).as_slice())? {
            Some(data) => Ok(deserialize(data)?),
            None => Err(GraphError::New(format!("Node not found: {}", id))),
        }
    }

    #[inline(always)]
    fn get_temp_edge(&self, txn: &RoTxn<'_>, id: &str) -> Result<Edge, GraphError> {
        match self.edges_db.get(&txn, Self::edge_key(id).as_slice())? {
            Some(data) => Ok(deserialize(data)?),
            None => Err(GraphError::New(format!("Edge not found: {}", id))),
        }
    }

    #[inline(always)]
    fn get_node(&self, txn: &RoTxn<'_>, id: &str) -> Result<Node, GraphError> {
        let n: Result<Node, GraphError> = match self.nodes_db.get(txn, &Self::node_key(id))? {
            Some(data) => Ok(deserialize(data)?),
            None => Err(GraphError::New(format!("Node not found: {}", id))),
        };
        n
    }

    #[inline(always)]
    fn get_edge(&self, txn: &RoTxn<'_>, id: &str) -> Result<Edge, GraphError> {
        let e: Result<Edge, GraphError> = match self.edges_db.get(txn, &Self::edge_key(id))? {
            Some(data) => Ok(deserialize(data)?),
            None => Err(GraphError::New(format!("Edge not found: {}", id))),
        };
        e
    }

    fn get_out_edges(
        &self,
        txn: &RoTxn<'_>,
        node_id: &str,
        edge_label: &str,
    ) -> Result<Vec<Edge>, GraphError> {
        let mut edges = Vec::with_capacity(512);

        let prefix = Self::out_edge_key(node_id, "");
        let iter = self.out_edges_db.prefix_iter(&txn, &prefix)?;

        for result in iter {
            let (key, _) = result?;
            let edge_id = std::str::from_utf8(&key[prefix.len()..])?;

            let edge = self.get_temp_edge(&txn, edge_id)?;
            if edge_label.is_empty() || edge.label == edge_label {
                edges.push(edge);
            }
        }

        Ok(edges)
    }

    fn get_in_edges(
        &self,
        txn: &RoTxn<'_>,
        node_id: &str,
        edge_label: &str,
    ) -> Result<Vec<Edge>, GraphError> {
        let mut edges = Vec::with_capacity(512);

        let prefix = Self::in_edge_key(node_id, "");
        let iter = self.in_edges_db.prefix_iter(&txn, &prefix)?;

        for result in iter {
            let (key, _) = result?;
            let edge_id = std::str::from_utf8(&key[prefix.len()..])?;

            let edge = self.get_temp_edge(&txn, edge_id)?;
            if edge_label.is_empty() || edge.label == edge_label {
                edges.push(edge);
            }
        }

        Ok(edges)
    }

    fn get_out_nodes(
        &self,
        txn: &RoTxn<'_>,
        node_id: &str,
        edge_label: &str,
    ) -> Result<Vec<Node>, GraphError> {
        let mut nodes = Vec::with_capacity(512);
        let prefix = Self::out_edge_key(node_id, "");
        let iter = self.out_edges_db.prefix_iter(txn, &prefix)?;

        for result in iter {
            let (key, _) = result?;
            let edge_id = std::str::from_utf8(&key[prefix.len()..])?;
            let edge = self.get_temp_edge(txn, edge_id)?;

            if edge_label.is_empty() || edge.label == edge_label {
                if let Ok(node) = self.get_temp_node(txn, &edge.to_node) {
                    nodes.push(node);
                }
            }
        }

        Ok(nodes)
    }

    fn get_in_nodes(
        &self,
        txn: &RoTxn<'_>,
        node_id: &str,
        edge_label: &str,
    ) -> Result<Vec<Node>, GraphError> {
        let mut nodes = Vec::with_capacity(512);
        let prefix = Self::in_edge_key(node_id, "");
        let iter = self.in_edges_db.prefix_iter(txn, &prefix)?;

        for result in iter {
            let (key, _) = result?;
            let edge_id = std::str::from_utf8(&key[prefix.len()..])?;
            let edge = self.get_temp_edge(txn, edge_id)?;

            if edge_label.is_empty() || edge.label == edge_label {
                if let Ok(node) = self.get_temp_node(txn, &edge.from_node) {
                    nodes.push(node);
                }
            }
        }

        Ok(nodes)
    }

    fn get_all_nodes(&self, txn: &RoTxn<'_>) -> Result<Vec<Node>, GraphError> {
        let mut nodes = Vec::with_capacity(self.nodes_db.len(txn)? as usize);

        let iter = self.nodes_db.iter(&txn)?;

        for result in iter {
            let (_, value) = result?;
            if !value.is_empty() {
                let node: Node = deserialize(value)?;
                nodes.push(node);
            }
        }

        Ok(nodes)
    }

    fn get_nodes_by_types(
        &self,
        txn: &RoTxn<'_>,
        types: &[String],
    ) -> Result<Vec<Node>, GraphError> {
        let mut nodes = Vec::new();

        for label in types {
            let prefix = [NODE_LABEL_PREFIX, label.as_bytes(), b":"].concat();
            let iter = self.node_labels_db.prefix_iter(&txn, &prefix)?;

            for result in iter {
                let (key, _) = result?;
                let node_id = std::str::from_utf8(&key[prefix.len()..])?;
                println!("Node ID: {}", node_id);

                let n: Result<Node, GraphError> =
                    match self.nodes_db.get(&txn, &Self::node_key(node_id))? {
                        Some(data) => Ok(deserialize(data)?),
                        None => Err(GraphError::New(format!("Node not found: {}", node_id))),
                    };
                println!("NODE: {:?}", n);
                if let Ok(node) = n {
                    println!("Node: {:?}", node);
                    nodes.push(node);
                }
            }
        }

        Ok(nodes)
    }

    fn get_all_edges(&self, txn: &RoTxn<'_>) -> Result<Vec<Edge>, GraphError> {
        let mut edges = Vec::with_capacity(self.edges_db.len(txn)? as usize);

        let iter = self.edges_db.iter(&txn)?;

        for result in iter {
            let (_, value) = result?;
            if !value.is_empty() {
                let edge: Edge = deserialize(value)?;
                edges.push(edge);
            }
        }

        Ok(edges)
    }

    fn create_node(
        &self,
        txn: &mut RwTxn<'_>,
        label: &str,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<Node, GraphError> {
        let node = Node {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            properties: HashMap::from_iter(properties),
        };

        // Store node data
        self.nodes_db
            .put(txn, &Self::node_key(&node.id), &serialize(&node)?)?;

        // Store node label index
        self.node_labels_db
            .put(txn, &Self::node_label_key(&node.label, &node.id), &())?;

        Ok(node)
    }

    fn create_edge(
        &self,
        txn: &mut RwTxn<'_>,
        label: &str,
        from_node: &str,
        to_node: &str,
        properties: impl IntoIterator<Item = (String, Value)>,
    ) -> Result<Edge, GraphError> {
        // Check if nodes exist

        // if self.check_exists(from_node)? || self.check_exists(to_node)? {
        //     return Err(GraphError::New(
        //         "One or both nodes do not exist".to_string(),
        //     ));
        // }

        if self
            .nodes_db
            .get(&txn, Self::node_key(from_node).as_slice())?
            .is_none()
            || self
                .nodes_db
                .get(&txn, Self::node_key(to_node).as_slice())?
                .is_none()
        {
            return Err(GraphError::NodeNotFound);
        }

        let edge = Edge {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            properties: HashMap::from_iter(properties),
        };

        // Store edge data
        self.edges_db
            .put(txn, &Self::edge_key(&edge.id), &serialize(&edge)?)?;

        // Store edge label index
        self.edge_labels_db
            .put(txn, &Self::edge_label_key(label, &edge.id), &())?;

        // Store edge - node maps
        self.out_edges_db
            .put(txn, &Self::out_edge_key(from_node, &edge.id), &())?;

        self.in_edges_db
            .put(txn, &Self::in_edge_key(to_node, &edge.id), &())?;

        Ok(edge)
    }

    fn drop_node(&self, txn: &mut RwTxn<'_>, id: &str) -> Result<(), GraphError> {
        // Get node to get its label
        let node = self.get_temp_node(&txn, id)?;

        // Delete outgoing edges
        let out_prefix = Self::out_edge_key(id, "");
        let mut out_edges = Vec::new();
        {
            let iter = self.out_edges_db.prefix_iter(&txn, &out_prefix)?;

            for result in iter {
                let (key, _) = result?;
                let edge_id = std::str::from_utf8(&key[out_prefix.len()..])?;

                if let Some(edge_data) = &self.edges_db.get(&txn, &Self::edge_key(edge_id))? {
                    let edge: Edge = deserialize(edge_data)?;
                    out_edges.push(edge);
                }
            }
        }

        // Delete incoming edges
        let in_prefix = Self::in_edge_key(id, "");
        let mut in_edges = Vec::new();
        {
            let iter = self.in_edges_db.prefix_iter(&txn, &in_prefix)?;

            for result in iter {
                let (key, _) = result?;
                let edge_id = std::str::from_utf8(&key[in_prefix.len()..])?;

                if let Some(edge_data) = self.edges_db.get(&txn, &Self::edge_key(edge_id))? {
                    let edge: Edge = deserialize(edge_data)?;
                    in_edges.push(edge);
                }
            }
        }

        // Delete all related data
        for edge in out_edges.iter().chain(in_edges.iter()) {
            // Delete edge data
            self.edges_db.delete(txn, &Self::edge_key(&edge.id))?;

            self.edge_labels_db
                .delete(txn, &Self::edge_label_key(&edge.label, &edge.id))?;
            self.out_edges_db
                .delete(txn, &Self::out_edge_key(&edge.from_node, &edge.id))?;
            self.in_edges_db
                .delete(txn, &Self::in_edge_key(&edge.to_node, &edge.id))?;
        }

        // Delete node data and label
        self.nodes_db.delete(txn, Self::node_key(id).as_slice())?;
        self.node_labels_db
            .delete(txn, &Self::node_label_key(&node.label, id))?;

        Ok(())
    }

    fn drop_edge(&self, txn: &mut RwTxn<'_>, edge_id: &str) -> Result<(), GraphError> {
        // Get edge data first
        let edge_data = match self.edges_db.get(&txn, &Self::edge_key(edge_id))? {
            Some(data) => data,
            None => return Err(GraphError::EdgeNotFound),
        };
        let edge: Edge = deserialize(edge_data)?;

        // Delete all edge-related data
        self.edges_db.delete(txn, &Self::edge_key(edge_id))?;
        self.edge_labels_db
            .delete(txn, &Self::edge_label_key(&edge.label, edge_id))?;
        self.out_edges_db
            .delete(txn, &Self::out_edge_key(&edge.from_node, edge_id))?;
        self.in_edges_db
            .delete(txn, &Self::in_edge_key(&edge.to_node, edge_id))?;

        Ok(())
    }
}

impl SearchMethods for HelixGraphStorage {
    fn shortest_path(
        &self,
        txn: &RoTxn<'_>,
        from_id: &str,
        to_id: &str,
    ) -> Result<(Vec<Node>, Vec<Edge>), GraphError> {
        let mut queue = VecDeque::with_capacity(32);
        let mut visited = HashSet::with_capacity(64);
        let mut parent = HashMap::with_capacity(32);
        queue.push_back(from_id.to_string());
        visited.insert(from_id.to_string());

        let reconstruct_path = |parent: &HashMap<String, (String, Edge)>,
                                start_id: &str,
                                end_id: &str|
         -> Result<(Vec<Node>, Vec<Edge>), GraphError> {
            let mut nodes = Vec::with_capacity(parent.len());
            let mut edges = Vec::with_capacity(parent.len() - 1);


            let mut current = end_id;

        while current != start_id {
            nodes.push(self.get_temp_node(txn, current)?);
            
            let (prev_node, edge) = &parent[current];
            edges.push(edge.clone());
            current = prev_node;
        }
        
        nodes.push(self.get_temp_node(txn, start_id)?);

            Ok((nodes, edges))
        };

        while let Some(current_id) = queue.pop_front() {
            let out_prefix = Self::out_edge_key(&current_id, "");
            let iter = self.out_edges_db.prefix_iter(&txn, &out_prefix)?;

            for result in iter {
                let (key, _) = result?;
                let edge_id = std::str::from_utf8(&key[out_prefix.len()..])?;

                let edge = self.get_temp_edge(&txn, edge_id)?;

                if !visited.contains(&edge.to_node) {
                    visited.insert(edge.to_node.clone());
                    parent.insert(edge.to_node.clone(), (current_id.to_string(), edge.clone()));
                    
                    if edge.to_node == to_id {
                        return reconstruct_path(&parent, from_id, to_id);
                    }
                    
                    queue.push_back(edge.to_node);
                }
            }
        }

        Err(GraphError::from(format!(
            "No path found between {} and {}",
            from_id, to_id
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::props;
    use crate::storage_core::storage_methods::StorageMethods;
    use protocol::value::Value;
    use tempfile::TempDir;

    fn setup_temp_db() -> HelixGraphStorage {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().to_str().unwrap();
        let storage = HelixGraphStorage::new(db_path).unwrap();

        storage
    }

    #[test]
    fn test_get_node() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let node = storage.create_node(&mut txn, "person", props! {}).unwrap();
        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        let retrieved_node = storage.get_node(&txn, &node.id).unwrap();

        // Compare after both transactions are complete
        assert_eq!(node.id, retrieved_node.id);
        assert_eq!(node.label, retrieved_node.label);
    }

    #[test]
    fn test_get_edge() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let node1 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node2 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let edge = storage
            .create_edge(&mut txn, "knows", &node1.id, &node2.id, props!())
            .unwrap(); // TODO: Handle Error

        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        let retrieved_edge = storage.get_edge(&txn, &edge.id).unwrap(); // TODO: Handle Error
        assert_eq!(edge.id, retrieved_edge.id);
        assert_eq!(edge.label, retrieved_edge.label);
        assert_eq!(edge.from_node, retrieved_edge.from_node);
        assert_eq!(edge.to_node, retrieved_edge.to_node);
    }

    #[test]
    fn test_create_node() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let properties = props! {
            "name" => "test node",
        };

        let node = storage.create_node(&mut txn, "person", properties).unwrap(); // TODO: Handle Error
        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        let retrieved_node = storage.get_node(&txn, &node.id).unwrap(); // TODO: Handle Error
        assert_eq!(node.id, retrieved_node.id);
        assert_eq!(node.label, "person");
        assert_eq!(
            node.properties.get("name").unwrap(),
            &Value::String("test node".to_string())
        );
    }

    #[test]
    fn test_create_edge() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let node1 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node2 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error

        let edge_props = props! {
            "age" => 22,
        };

        let edge = storage
            .create_edge(&mut txn, "knows", &node1.id, &node2.id, edge_props)
            .unwrap(); // TODO: Handle Error

        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        let retrieved_edge = storage.get_edge(&txn, &edge.id).unwrap(); // TODO: Handle Error
        assert_eq!(edge.id, retrieved_edge.id);
        assert_eq!(edge.label, "knows");
        assert_eq!(edge.from_node, node1.id);
        assert_eq!(edge.to_node, node2.id);
    }

    #[test]
    fn test_create_edge_with_nonexistent_nodes() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let result =
            storage.create_edge(&mut txn, "knows", "nonexistent1", "nonexistent2", props!());

        assert!(result.is_err());
    }

    #[test]
    fn test_drop_node() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let node1 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node2 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node3 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error

        storage
            .create_edge(&mut txn, "knows", &node1.id, &node2.id, props!())
            .unwrap(); // TODO: Handle Error
        storage
            .create_edge(&mut txn, "knows", &node3.id, &node1.id, props!())
            .unwrap(); // TODO: Handle Error

        storage.drop_node(&mut txn, &node1.id).unwrap(); // TODO: Handle Error
        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        assert!(storage.get_node(&txn, &node1.id).is_err());
    }

    #[test]
    fn test_drop_edge() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let node1 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node2 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let edge = storage
            .create_edge(&mut txn, "knows", &node1.id, &node2.id, props!())
            .unwrap(); // TODO: Handle Error

        storage.drop_edge(&mut txn, &edge.id).unwrap(); // TODO: Handle Error

        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        assert!(storage.get_edge(&txn, &edge.id).is_err());
    }

    #[test]
    fn test_check_exists() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let node = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        assert!(storage.check_exists(&txn, &node.id).unwrap());
        assert!(!storage.check_exists(&txn, "nonexistent").unwrap());
    }

    #[test]
    fn test_multiple_edges_between_nodes() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let node1 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node2 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error

        let edge1 = storage
            .create_edge(&mut txn, "knows", &node1.id, &node2.id, props!())
            .unwrap(); // TODO: Handle Error
        let edge2 = storage
            .create_edge(&mut txn, "likes", &node1.id, &node2.id, props!())
            .unwrap(); // TODO: Handle Error

        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        assert!(storage.get_edge(&txn, &edge1.id).is_ok());
        assert!(storage.get_edge(&txn, &edge2.id).is_ok());
    }

    #[test]
    fn test_node_with_properties() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let properties = props! {
            "name" => "George",
            "age" => 22,
            "active" => true,
        };
        let node = storage.create_node(&mut txn, "person", properties).unwrap(); // TODO: Handle Error
        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        let retrieved_node = storage.get_node(&txn, &node.id).unwrap(); // TODO: Handle Error

        assert_eq!(
            retrieved_node.properties.get("name").unwrap(),
            &Value::String("George".to_string())
        );
        assert!(match retrieved_node.properties.get("age").unwrap() {
            Value::Integer(val) => val == &22,
            Value::Float(val) => val == &22.0,
            _ => false,
        });
        assert_eq!(
            retrieved_node.properties.get("active").unwrap(),
            &Value::Boolean(true)
        );
    }

    #[test]
    fn test_get_all_nodes() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();
        let node1 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node2 = storage.create_node(&mut txn, "thing", props!()).unwrap(); // TODO: Handle Error
        let node3 = storage.create_node(&mut txn, "other", props!()).unwrap(); // TODO: Handle Error

        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        let nodes = storage.get_all_nodes(&txn).unwrap(); // TODO: Handle Error

        assert_eq!(nodes.len(), 3);

        let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();

        assert!(node_ids.contains(&node1.id));
        assert!(node_ids.contains(&node2.id));
        assert!(node_ids.contains(&node3.id));

        let labels: Vec<String> = nodes.iter().map(|n| n.label.clone()).collect();

        assert!(labels.contains(&"person".to_string()));
        assert!(labels.contains(&"thing".to_string()));
        assert!(labels.contains(&"other".to_string()));
    }

    #[test]
    fn test_get_all_node_by_types() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();
        let node1 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node2 = storage.create_node(&mut txn, "thing", props!()).unwrap(); // TODO: Handle Error
        let node3 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        println!("node1: {:?}, node2: {:?}, node3: {:?}", node1, node2, node3);

        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        let nodes = storage
            .get_nodes_by_types(&txn, &["person".to_string()])
            .unwrap(); // TODO: Handle Error

        assert_eq!(nodes.len(), 2);

        let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();

        assert!(node_ids.contains(&node1.id));
        assert!(!node_ids.contains(&node2.id));
        assert!(node_ids.contains(&node3.id));
    }

    #[test]
    fn test_get_all_edges() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();

        let node1 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node2 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error
        let node3 = storage.create_node(&mut txn, "person", props!()).unwrap(); // TODO: Handle Error

        let edge1 = storage
            .create_edge(&mut txn, "knows", &node1.id, &node2.id, props!())
            .unwrap(); // TODO: Handle Error
        let edge2 = storage
            .create_edge(&mut txn, "likes", &node2.id, &node3.id, props!())
            .unwrap(); // TODO: Handle Error
        let edge3 = storage
            .create_edge(&mut txn, "follows", &node1.id, &node3.id, props!())
            .unwrap(); // TODO: Handle Error

        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        let edges = storage.get_all_edges(&txn).unwrap(); // TODO: Handle Error

        assert_eq!(edges.len(), 3);

        let edge_ids: Vec<String> = edges.iter().map(|e| e.id.clone()).collect();

        assert!(edge_ids.contains(&edge1.id));
        assert!(edge_ids.contains(&edge2.id));
        assert!(edge_ids.contains(&edge3.id));

        let labels: Vec<String> = edges.iter().map(|e| e.label.clone()).collect();

        assert!(labels.contains(&"knows".to_string()));
        assert!(labels.contains(&"likes".to_string()));
        assert!(labels.contains(&"follows".to_string()));

        let connections: Vec<(String, String)> = edges
            .iter()
            .map(|e| (e.from_node.clone(), e.to_node.clone()))
            .collect();

        assert!(connections.contains(&(node1.id.clone(), node2.id.clone())));
        assert!(connections.contains(&(node2.id.clone(), node3.id.clone())));
        assert!(connections.contains(&(node1.id.clone(), node3.id.clone())));
    }

    #[test]
    fn test_shortest_path() {
        let storage = setup_temp_db();
        let mut txn = storage.env.write_txn().unwrap();
        let mut nodes = Vec::new();
        for _ in 0..6 {
            let node = storage.create_node(&mut txn, "person", props!()).unwrap();
            nodes.push(node);
        }

        storage
            .create_edge(&mut txn, "knows", &nodes[0].id, &nodes[1].id, props!())
            .unwrap();
        storage
            .create_edge(&mut txn, "knows", &nodes[0].id, &nodes[2].id, props!())
            .unwrap();

        storage
            .create_edge(&mut txn, "knows", &nodes[1].id, &nodes[3].id, props!())
            .unwrap();
        storage
            .create_edge(&mut txn, "knows", &nodes[1].id, &nodes[2].id, props!())
            .unwrap();

        storage
            .create_edge(&mut txn, "knows", &nodes[2].id, &nodes[1].id, props!())
            .unwrap();
        storage
            .create_edge(&mut txn, "knows", &nodes[2].id, &nodes[3].id, props!())
            .unwrap();
        storage
            .create_edge(&mut txn, "knows", &nodes[2].id, &nodes[4].id, props!())
            .unwrap();

        storage
            .create_edge(&mut txn, "knows", &nodes[4].id, &nodes[3].id, props!())
            .unwrap();
        storage
            .create_edge(&mut txn, "knows", &nodes[4].id, &nodes[5].id, props!())
            .unwrap();

        txn.commit().unwrap();

        let txn = storage.env.read_txn().unwrap();
        let shortest_path1 = storage
            .shortest_path(&txn, &nodes[0].id, &nodes[5].id)
            .unwrap()
            .1
            .len();
        let shortest_path2 = storage
            .shortest_path(&txn, &nodes[1].id, &nodes[5].id)
            .unwrap()
            .1
            .len();
        assert_eq!(shortest_path1, 3);
        assert_eq!(shortest_path2, 3);
    }
}
