#[cfg(test)]
mod tests {
    use rand::seq::SliceRandom;
    use rand::rng;
    use rust_order_book::enums::Side;
    use rust_order_book::order_side::OrderSide;
    use uuid::Uuid;

    fn create_orderside(side: Side) -> OrderSide {
        OrderSide::new(side)
    }

    #[test]
    fn test_append_and_remove() {
        let mut os = create_orderside(Side::Buy);
        let price = 1000;
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        os.append(id1, 50, price);
        os.append(id2, 70, price);

        assert_eq!(os.volume, 120);
        assert_eq!(os.prices.get(&price).unwrap().volume, 120);

        let mut queue = os.take_queue(price).unwrap();
        os.remove(id1, 50, price, &mut queue);
        assert_eq!(os.volume, 70);

        os.put_queue(price, queue);
        let mut queue = os.take_queue(price).unwrap();
        os.remove(id2, 70, price, &mut queue);
        assert!(os.is_empty());
        assert_eq!(os.volume, 0);
    }

    #[test]
    fn test_best_min_max_price() {
        let mut os = create_orderside(Side::Sell);
        os.append(Uuid::new_v4(), 10, 100);
        os.append(Uuid::new_v4(), 20, 200);
        os.append(Uuid::new_v4(), 30, 150);

        assert_eq!(os.min_price(), Some(100));
        assert_eq!(os.max_price(), Some(200));
        assert_eq!(os.best_price(true), Some(100));
        assert_eq!(os.best_price(false), Some(200));
    }

    #[test]
    fn test_depth() {
        let mut os = create_orderside(Side::Buy);
        os.append(Uuid::new_v4(), 10, 100);
        os.append(Uuid::new_v4(), 20, 200);
        os.append(Uuid::new_v4(), 30, 150);

        let d = os.depth(2);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].0, 200);
        assert_eq!(d[1].0, 150);
    }

    #[test]
    fn test_display_buy() {
        let mut side = OrderSide::new(Side::Buy);
        side.append(Uuid::new_v4(), 100, 10);
        side.append(Uuid::new_v4(), 200, 20);

        let output = format!("{}", side);
        assert!(output.contains("10 -> 100"));
        assert!(output.contains("20 -> 200"));
    }

    #[test]
    fn test_display_sell() {
        let mut side = OrderSide::new(Side::Sell);
        side.append(Uuid::new_v4(), 50, 5);
        side.append(Uuid::new_v4(), 150, 15);

        let output = format!("{}", side);
        // Per Side::Sell l'iteratore viene invertito
        assert!(output.contains("15 -> 150"));
        assert!(output.contains("5 -> 50"));
    }

    #[test]
    fn test_display_empty() {
        let side = OrderSide::new(Side::Buy);
        let output = format!("{}", side);
        assert!(output.is_empty());
    }

    #[test]
    fn stress_test_random() {
        let mut os = OrderSide::new(Side::Buy);
        let mut rng = rng();
        let mut orders = Vec::new();

        // aggiungo 1000 ordini su prezzi casuali tra 100..200
        for _ in 0..1000 {
            let id = Uuid::new_v4();
            let qty = rand::random::<u128>() % 500 + 1;
            let price = 100 + rand::random::<u128>() % 100;
            os.append(id, qty, price);
            orders.push((id, qty, price));
        }

        // aggiorno metà degli ordini con quantità casuali
        let mut to_update = orders.clone();
        to_update.shuffle(&mut rng);
        for (id, _, price) in to_update.iter().take(500) {
            if let Some(mut queue) = os.take_queue(*price) {
                if queue.iter_ids().iter().any(|x| *x == *id) {
                    let new_qty = rand::random::<u128>() % 500 + 1;
                    queue.update(*id, 0, new_qty); // aggiorna quantità
                }
                os.put_queue(*price, queue);
            }
        }

        // rimuovo tutti gli ordini in ordine casuale
        orders.shuffle(&mut rng);
        for (id, qty, price) in orders.iter() {
            if let Some(mut queue) = os.take_queue(*price) {
                if queue.iter_ids().iter().any(|x| *x == *id) {
                    os.remove(*id, *qty, *price, &mut queue);
                }
                if !queue.is_empty() {
                    os.put_queue(*price, queue);
                }
            }
        }

        assert!(os.is_empty());
        assert_eq!(os.volume, 0);
    }
}