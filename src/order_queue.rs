use std::collections::{HashMap};
use uuid::Uuid;

use crate::math::math::{safe_add, safe_add_sub, safe_sub};


#[derive(Debug)]
struct Node {
    prev: Option<Uuid>,
    next: Option<Uuid>,
    quantity: u128
}

#[derive(Debug)]
pub struct OrderQueue {
    pub price: u128,
    pub volume: u128,
    head: Option<Uuid>,
    tail: Option<Uuid>,
    nodes: HashMap<Uuid, Node>
}

impl OrderQueue {
    pub fn new(price: u128) -> OrderQueue {
        OrderQueue { 
            price,
            volume: 0,
            head: None,
            tail: None,
            nodes: HashMap::new()
        }
    }

    pub fn is_empty(&self) -> bool { self.head.is_none() }
    pub fn is_not_empty(&self) -> bool { self.head.is_some() }
    pub fn head(&self) -> Option<Uuid> { self.head }
    pub fn tail(&self) -> Option<Uuid> { self.tail }

    /// Add the order id to the tail of the queue
    pub fn append(&mut self, id: Uuid, quantity: u128) {
        let new = Node { prev: self.tail, next: None, quantity };
        self.volume = safe_add(self.volume, quantity);
        
        if let Some(tail_id) = self.tail {
            if let Some(tail_node) = self.nodes.get_mut(&tail_id) {
                tail_node.next = Some(id);
            }
        } else {
            // First element
            self.head = Some(id);
        }

        self.tail = Some(id);
        self.nodes.insert(id, new);
    }

    // sets up new order to list value
    pub fn update (&mut self, id: Uuid, old_quantity: u128, new_quantity: u128) {
        if let Some(node) = self.nodes.get_mut(&id) {
            self.volume = safe_add_sub(self.volume, new_quantity, old_quantity);
            node.quantity = new_quantity;
        }
	}

    /// removes order from the queue
    pub fn remove(&mut self, id: Uuid, quantity: u128) {
        let node = match self.nodes.remove(&id) {
            Some(n) => n,
            None => return,
        };

        self.volume = safe_sub(self.volume, quantity);

        match (node.prev, node.next) {
            (Some(prev), Some(next)) => {
                self.nodes.get_mut(&prev).unwrap().next = Some(next);
                self.nodes.get_mut(&next).unwrap().prev = Some(prev);
            }
            (Some(prev), None) => {
                self.nodes.get_mut(&prev).unwrap().next = None;
                self.tail = Some(prev);
            }
            (None, Some(next)) => {
                self.nodes.get_mut(&next).unwrap().prev = None;
                self.head = Some(next);
            }
            (None, None) => {
                self.head = None;
                self.tail = None;
            }
        }
    }

    pub fn iter_ids(&self) -> Vec<Uuid> {
        let mut ids = Vec::new();
        let mut current = self.head;
        while let Some(id) = current {
            ids.push(id);
            current = self.nodes.get(&id).and_then(|n| n.next);
        }
        ids
    }
}