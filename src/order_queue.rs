use std::collections::VecDeque;

use crate::math::math::{safe_add, safe_add_sub, safe_sub};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) struct OrderQueue {
    pub volume: u64,
    orders: VecDeque<Uuid>,
}

impl OrderQueue {
    pub fn new() -> OrderQueue {
        OrderQueue { volume: 0, orders: VecDeque::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.orders.len() == 0
    }

    pub fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }

    pub fn head(&self) -> Option<Uuid> {
        self.orders.front().copied()
    }

    /// Add the order id to the tail of the queue
    pub fn append(&mut self, id: Uuid, quantity: u64) {
        self.volume = safe_add(self.volume, quantity);
        self.orders.push_back(id);
    }

    /// sets up new order to list value
    pub fn update(&mut self, id: Uuid, old_quantity: u64, new_quantity: u64) {
        self.volume = safe_add_sub(self.volume, new_quantity, old_quantity);
    }

    /// removes order from the queue
    pub fn remove(&mut self, id: Uuid, quantity: u64) {
        if let Some(pos) = self.orders.iter().position(|&x| x == id) {
            self.orders.remove(pos);
            self.volume = safe_sub(self.volume, quantity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::new_order_id;
    use rand::rng;
    use rand::seq::SliceRandom;

    fn make_uuid() -> Uuid {
        new_order_id()
    }

    #[test]
    fn test_new_queue_is_empty() {
        let q = OrderQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.volume, 0);
        assert_eq!(q.orders.len(), 0);
    }

    #[test]
    fn test_append_one_order() {
        let mut q = OrderQueue::new();
        let id = make_uuid();
        q.append(id, 50);

        assert!(q.is_not_empty());
        assert_eq!(q.volume, 50);
        assert_eq!(q.orders.get(0).cloned(), Some(id));
    }

    #[test]
    fn test_append_multiple_orders() {
        let mut q = OrderQueue::new();
        let id1 = make_uuid();
        let id2 = make_uuid();
        let id3 = make_uuid();

        q.append(id1, 10);
        q.append(id2, 20);
        q.append(id3, 30);

        assert_eq!(q.volume, 60);
        let items: Vec<Uuid> = q.orders.iter().map(|x| *x).collect();
        assert_eq!(items, vec![id1, id2, id3]);
    }

    #[test]
    fn test_update_order() {
        let mut q = OrderQueue::new();
        let id = make_uuid();
        q.append(id, 10);

        q.update(id, 10, 25);
        assert_eq!(q.volume, 25);
    }

    // #[test]
    // fn test_update_order_that_not_exists() {
    //     let mut q = OrderQueue::new();
    //     let id = make_uuid();
    //     q.append(id, 10);

    //     q.update(make_uuid(), 10, 25);
    //     assert_eq!(q.volume, 10);
    //     assert_eq!(q.nodes.get(&id).unwrap().quantity, 10);
    // }

    // #[test]
    // fn test_remove_middle_order() {
    //     let mut q = OrderQueue::new();
    //     let id1 = make_uuid();
    //     let id2 = make_uuid();
    //     let id3 = make_uuid();

    //     q.append(id1, 10);
    //     q.append(id2, 20);
    //     q.append(id3, 30);

    //     q.remove(id2, 20);

    //     assert_eq!(q.volume, 40); // 10 + 30
    //     assert_eq!(iter_ids(&q), vec![id1, id3]);
    //     assert_eq!(q.head(), Some(id1));
    //     assert_eq!(q.tail(), Some(id3));
    // }

    // #[test]
    // fn test_remove_head_order() {
    //     let mut q = OrderQueue::new();
    //     let id1 = make_uuid();
    //     let id2 = make_uuid();

    //     q.append(id1, 10);
    //     q.append(id2, 20);

    //     q.remove(id1, 10);

    //     assert_eq!(q.volume, 20);
    //     assert_eq!(q.head(), Some(id2));
    //     assert_eq!(q.tail(), Some(id2));
    //     assert_eq!(iter_ids(&q), vec![id2]);
    // }

    // #[test]
    // fn test_remove_tail_order() {
    //     let mut q = OrderQueue::new();
    //     let id1 = make_uuid();
    //     let id2 = make_uuid();

    //     q.append(id1, 10);
    //     q.append(id2, 20);

    //     q.remove(id2, 20);

    //     assert_eq!(q.volume, 10);
    //     assert_eq!(q.head(), Some(id1));
    //     assert_eq!(q.tail(), Some(id1));
    //     assert_eq!(iter_ids(&q), vec![id1]);
    // }

    // #[test]
    // fn test_remove_only_order() {
    //     let mut q = OrderQueue::new();
    //     let id = make_uuid();

    //     q.append(id, 50);
    //     q.remove(id, 50);

    //     assert!(q.is_empty());
    //     assert_eq!(q.volume, 0);
    //     assert_eq!(iter_ids(&q).len(), 0);
    // }

    // #[test]
    // fn test_order_that_not_exist() {
    //     let mut q = OrderQueue::new();
    //     let id = make_uuid();

    //     q.append(id, 50);
    //     q.remove(make_uuid(), 50);

    //     assert!(q.is_not_empty());
    //     assert_eq!(q.volume, 50);
    //     assert_eq!(iter_ids(&q).len(), 1);
    // }

    // #[test]
    // fn stress_test_append_and_remove() {
    //     let mut q = OrderQueue::new();

    //     let mut ids = Vec::new();
    //     let n = 1000;

    //     // inserisco 1000 ordini con quantità = indice+1
    //     for i in 0..n {
    //         let id = new_order_id();
    //         q.append(id, (i + 1) as u64);
    //         ids.push(id);
    //     }

    //     // volume atteso = somma 1..=n = n*(n+1)/2
    //     let expected_volume: u64 = (n as u64) * ((n as u64) + 1) / 2;
    //     assert_eq!(q.volume, expected_volume);
    //     assert_eq!(iter_ids(&q).len(), n);

    //     // rimuovo tutti gli ordini
    //     for (i, id) in ids.iter().enumerate() {
    //         q.remove(*id, (i + 1) as u64);
    //     }

    //     assert!(q.is_empty());
    //     assert_eq!(q.volume, 0);
    //     assert_eq!(iter_ids(&q).len(), 0);
    // }

    // #[test]
    // fn random_append_remove_test() {
    //     let mut q = OrderQueue::new();
    //     let mut ids = Vec::new();
    //     let mut rng = rng();

    //     // aggiungo 500 ordini con quantità casuale 1..1000
    //     for _ in 0..500 {
    //         let id = new_order_id();
    //         let qty = rand::random::<u64>() % 1000 + 1;
    //         q.append(id, qty);
    //         ids.push((id, qty));
    //     }

    //     // controllo volume totale
    //     let expected_volume: u64 = ids.iter().map(|(_, qty)| *qty).sum();
    //     assert_eq!(q.volume, expected_volume);

    //     // rimuovo gli ordini in ordine casuale
    //     ids.shuffle(&mut rng);
    //     for (id, qty) in ids.iter() {
    //         q.remove(*id, *qty);
    //     }

    //     // alla fine la coda deve essere vuota
    //     assert!(q.is_empty());
    //     assert_eq!(q.volume, 0);
    //     assert_eq!(iter_ids(&q).len(), 0);
    // }

    // #[test]
    // fn random_update_test() {
    //     let mut q = OrderQueue::new();
    //     let mut ids = Vec::new();
    //     let mut rng = rng();

    //     // aggiungo 300 ordini con quantità casuale 1..500
    //     for _ in 0..300 {
    //         let id = new_order_id();
    //         let qty = rand::random::<u64>() % 500 + 1;
    //         q.append(id, qty);
    //         ids.push((id, qty));
    //     }

    //     // aggiorno casualmente circa metà degli ordini
    //     let mut ids_to_update = ids.clone();
    //     ids_to_update.shuffle(&mut rng);
    //     let updates = &ids_to_update[..150];

    //     for (id, old_qty) in updates.iter() {
    //         let new_qty = rand::random::<u64>() % 500 + 1;
    //         q.update(*id, *old_qty, new_qty);
    //         // aggiorno anche la quantità locale per il calcolo volume

    //         let pos = ids.iter().position(|(i, _)| i == id);
    //         ids[pos.unwrap()].1 = new_qty;
    //     }

    //     // controllo volume totale
    //     let expected_volume: u64 = ids.iter().map(|(_, qty)| *qty).sum();
    //     assert_eq!(q.volume, expected_volume);

    //     // rimuovo tutti gli ordini in ordine casuale
    //     ids.shuffle(&mut rng);
    //     for (id, qty) in ids.iter() {
    //         q.remove(*id, *qty);
    //     }

    //     // alla fine la coda deve essere vuota
    //     assert!(q.is_empty());
    //     assert_eq!(q.volume, 0);
    //     assert_eq!(iter_ids(&q).len(), 0);
    // }
}
