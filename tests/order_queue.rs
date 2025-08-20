#[cfg(test)]
mod tests {
    use rust_order_book::order_queue::OrderQueue;
    use uuid::Uuid;
    use rand::seq::SliceRandom;
    use rand::rng;

    fn make_uuid() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn test_new_queue_is_empty() {
        let q = OrderQueue::new(100);
        assert!(q.is_empty());
        assert_eq!(q.volume, 0);
        assert_eq!(q.head(), None);
        assert_eq!(q.tail(), None);
        assert_eq!(q.iter_ids().len(), 0);
    }

    #[test]
    fn test_append_one_order() {
        let mut q = OrderQueue::new(100);
        let id = make_uuid();
        q.append(id, 50);

        assert!(q.is_not_empty());
        assert_eq!(q.volume, 50);
        assert_eq!(q.head(), Some(id));
        assert_eq!(q.tail(), Some(id));
        assert_eq!(q.iter_ids(), vec![id]);
    }

    #[test]
    fn test_append_multiple_orders() {
        let mut q = OrderQueue::new(100);
        let id1 = make_uuid();
        let id2 = make_uuid();
        let id3 = make_uuid();

        q.append(id1, 10);
        q.append(id2, 20);
        q.append(id3, 30);

        assert_eq!(q.volume, 60);
        assert_eq!(q.head(), Some(id1));
        assert_eq!(q.tail(), Some(id3));
        assert_eq!(q.iter_ids(), vec![id1, id2, id3]);
    }

    #[test]
    fn test_update_order() {
        let mut q = OrderQueue::new(100);
        let id = make_uuid();
        q.append(id, 10);

        q.update(id, 10, 25);
        assert_eq!(q.volume, 25);
        assert_eq!(q.nodes.get(&id).unwrap().quantity, 25);
    }

    #[test]
    fn test_update_order_that_not_exists() {
        let mut q = OrderQueue::new(100);
        let id = make_uuid();
        q.append(id, 10);

        q.update(make_uuid(), 10, 25);
        assert_eq!(q.volume, 10);
        assert_eq!(q.nodes.get(&id).unwrap().quantity, 10);
    }

    #[test]
    fn test_remove_middle_order() {
        let mut q = OrderQueue::new(100);
        let id1 = make_uuid();
        let id2 = make_uuid();
        let id3 = make_uuid();

        q.append(id1, 10);
        q.append(id2, 20);
        q.append(id3, 30);

        q.remove(id2, 20);

        assert_eq!(q.volume, 40); // 10 + 30
        assert_eq!(q.iter_ids(), vec![id1, id3]);
        assert_eq!(q.head(), Some(id1));
        assert_eq!(q.tail(), Some(id3));
    }

    #[test]
    fn test_remove_head_order() {
        let mut q = OrderQueue::new(100);
        let id1 = make_uuid();
        let id2 = make_uuid();

        q.append(id1, 10);
        q.append(id2, 20);

        q.remove(id1, 10);

        assert_eq!(q.volume, 20);
        assert_eq!(q.head(), Some(id2));
        assert_eq!(q.tail(), Some(id2));
        assert_eq!(q.iter_ids(), vec![id2]);
    }

    #[test]
    fn test_remove_tail_order() {
        let mut q = OrderQueue::new(100);
        let id1 = make_uuid();
        let id2 = make_uuid();

        q.append(id1, 10);
        q.append(id2, 20);

        q.remove(id2, 20);

        assert_eq!(q.volume, 10);
        assert_eq!(q.head(), Some(id1));
        assert_eq!(q.tail(), Some(id1));
        assert_eq!(q.iter_ids(), vec![id1]);
    }

    #[test]
    fn test_remove_only_order() {
        let mut q = OrderQueue::new(100);
        let id = make_uuid();

        q.append(id, 50);
        q.remove(id, 50);

        assert!(q.is_empty());
        assert_eq!(q.volume, 0);
        assert_eq!(q.iter_ids().len(), 0);
    }

    #[test]
    fn test_order_that_not_exist() {
        let mut q = OrderQueue::new(100);
        let id = make_uuid();

        q.append(id, 50);
        q.remove(make_uuid(), 50);

        assert!(q.is_not_empty());
        assert_eq!(q.volume, 50);
        assert_eq!(q.iter_ids().len(), 1);
    }

    #[test]
    fn stress_test_append_and_remove() {
        let mut q = OrderQueue::new(100);

        let mut ids = Vec::new();
        let n = 1000;

        // inserisco 1000 ordini con quantità = indice+1
        for i in 0..n {
            let id = Uuid::new_v4();
            q.append(id, (i + 1) as u128);
            ids.push(id);
        }

        // volume atteso = somma 1..=n = n*(n+1)/2
        let expected_volume: u128 = (n as u128) * ((n as u128) + 1) / 2;
        assert_eq!(q.volume, expected_volume);
        assert_eq!(q.iter_ids().len(), n);

        // rimuovo tutti gli ordini
        for (i, id) in ids.iter().enumerate() {
            q.remove(*id, (i + 1) as u128);
        }

        assert!(q.is_empty());
        assert_eq!(q.volume, 0);
        assert_eq!(q.iter_ids().len(), 0);
    }

    #[test]
    fn random_append_remove_test() {
        let mut q = OrderQueue::new(50);
        let mut ids = Vec::new();
        let mut rng = rng();

        // aggiungo 500 ordini con quantità casuale 1..1000
        for _ in 0..500 {
            let id = Uuid::new_v4();
            let qty = rand::random::<u128>() % 1000 + 1;
            q.append(id, qty);
            ids.push((id, qty));
        }

        // controllo volume totale
        let expected_volume: u128 = ids.iter().map(|(_, qty)| *qty).sum();
        assert_eq!(q.volume, expected_volume);

        // rimuovo gli ordini in ordine casuale
        ids.shuffle(&mut rng);
        for (id, qty) in ids.iter() {
            q.remove(*id, *qty);
        }

        // alla fine la coda deve essere vuota
        assert!(q.is_empty());
        assert_eq!(q.volume, 0);
        assert_eq!(q.iter_ids().len(), 0);
    }

    #[test]
    fn random_update_test() {
        let mut q = OrderQueue::new(200);
        let mut ids = Vec::new();
        let mut rng = rng();

        // aggiungo 300 ordini con quantità casuale 1..500
        for _ in 0..300 {
            let id = Uuid::new_v4();
            let qty = rand::random::<u128>() % 500 + 1;
            q.append(id, qty);
            ids.push((id, qty));
        }

        // aggiorno casualmente circa metà degli ordini
        let mut ids_to_update = ids.clone();
        ids_to_update.shuffle(&mut rng);
        let updates = &ids_to_update[..150];

        for (id, old_qty) in updates.iter() {
            let new_qty = rand::random::<u128>() % 500 + 1;
            q.update(*id, *old_qty, new_qty);
            // aggiorno anche la quantità locale per il calcolo volume
            if let Some(pos) = ids.iter().position(|(i, _)| i == id) {
                ids[pos].1 = new_qty;
            }
        }

        // controllo volume totale
        let expected_volume: u128 = ids.iter().map(|(_, qty)| *qty).sum();
        assert_eq!(q.volume, expected_volume);

        // rimuovo tutti gli ordini in ordine casuale
        ids.shuffle(&mut rng);
        for (id, qty) in ids.iter() {
            q.remove(*id, *qty);
        }

        // alla fine la coda deve essere vuota
        assert!(q.is_empty());
        assert_eq!(q.volume, 0);
        assert_eq!(q.iter_ids().len(), 0);
    }
}