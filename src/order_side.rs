use rb_tree::RBQueue;
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::cmp::Ordering;
use std::fmt;
use uuid::Uuid;

use crate::enums::Side;
use crate::math::math::{safe_add, safe_sub};
use crate::order_queue::OrderQueue;

#[derive(Debug)]
pub(crate) struct OrderSide {
    pub prices_tree: RBQueue<u64, fn(&u64, &u64) -> Ordering>,
    pub prices: FxHashMap<u64, OrderQueue>,
    pub volume: u64,
    side: Side,
}

impl OrderSide {
    pub fn with_capacity(side: Side, capacity: usize) -> Self {
        Self {
            side,
            prices: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher),
            prices_tree: RBQueue::<u64, _>::new(if side == Side::Buy {
                OrderSide::buy_cmp
            } else {
                OrderSide::sell_cmp
            }),
            volume: 0,
        }
    }

    fn buy_cmp(a: &u64, b: &u64) -> Ordering {
        b.cmp(a) // ordine decrescente
    }

    fn sell_cmp(a: &u64, b: &u64) -> Ordering {
        a.cmp(b) // ordine crescente
    }

    // appends order to definite price level
    pub fn append(&mut self, id: Uuid, quantity: u64, price: u64) {
        if !self.prices.contains_key(&price) {
            self.prices.insert(price, OrderQueue::new(price));
            self.prices_tree.insert(price);
        }

        let queue = self.prices.get_mut(&price).expect(
            format!("OrderSide on {:?} is broken: price {} not in prices", self.side, price)
                .as_str(),
        );
        self.volume = safe_add(self.volume, quantity);
        queue.append(id, quantity);
    }

    // removes order from definite price level
    pub fn remove(&mut self, id: Uuid, quantity: u64, price: u64, queue: &mut OrderQueue) {
        queue.remove(id, quantity);
        if queue.is_empty() {
            self.prices.remove(&price);
            self.prices_tree.remove(&price);
        }
        self.volume = safe_sub(self.volume, quantity);
    }

    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }

    pub fn take_queue(&mut self, price: u64) -> Option<OrderQueue> {
        self.prices.remove(&price)
    }

    pub fn put_queue(&mut self, price: u64, q: OrderQueue) {
        self.prices.insert(price, q);
    }

    pub fn best_price(&self, min: bool) -> Option<u64> {
        let price = match (self.side, min) {
            (Side::Sell, true) | (Side::Buy, false) => self.prices_tree.peek(),
            (Side::Sell, false) | (Side::Buy, true) => self.prices_tree.peek_back(),
        };
        price.copied()
    }

    // returns max level of price
    pub fn min_price(&self) -> Option<u64> {
        self.best_price(true)
    }

    // returns min level of price
    pub fn max_price(&self) -> Option<u64> {
        self.best_price(false)
    }

    pub fn depth(&self, limit: u32) -> Vec<(u64, u64)> {
        let mut depth = Vec::new();
        let mut count = 0;

        for price in self.prices_tree.ordered() {
            if count >= limit {
                break;
            }

            let queue = self.prices.get(price)
                .expect(format!("[dept()]: In OrderSide {:?} the price {} is in price_tree but is missing in the prices map", self.side, price).as_str());
            depth.push((*price, queue.volume));
            count += 1;
        }

        depth
    }
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prices = self.prices_tree.ordered();
        let iter: Box<dyn Iterator<Item = &&u64>> = match self.side {
            Side::Sell => Box::new(prices.iter().rev()),
            Side::Buy => Box::new(prices.iter()),
        };

        for price in iter {
            let queue = self.prices.get(&price)
                .expect(format!("[fmt::Display]: In OrderSide {:?} the price {} is in price_tree but is missing in the prices map", self.side, price).as_str());
            writeln!(f, "{} -> {}", price, queue.volume).expect("Failed to write to formatter");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::new_order_id;

    use super::*;
    use rand::rng;
    use rand::seq::SliceRandom;

    fn create_orderside(side: Side) -> OrderSide {
        OrderSide::with_capacity(side, 10)
    }

    #[test]
    fn test_append_and_remove() {
        let mut os = create_orderside(Side::Buy);
        let price = 1000;
        let id1 = new_order_id();
        let id2 = new_order_id();

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
        os.append(new_order_id(), 10, 100);
        os.append(new_order_id(), 20, 200);
        os.append(new_order_id(), 30, 150);

        assert_eq!(os.min_price(), Some(100));
        assert_eq!(os.max_price(), Some(200));
        assert_eq!(os.best_price(true), Some(100));
        assert_eq!(os.best_price(false), Some(200));
    }

    #[test]
    fn test_depth() {
        let mut os = create_orderside(Side::Buy);
        os.append(new_order_id(), 10, 100);
        os.append(new_order_id(), 20, 200);
        os.append(new_order_id(), 30, 150);

        let d = os.depth(2);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].0, 200);
        assert_eq!(d[1].0, 150);
    }

    #[test]
    fn test_display_buy() {
        let mut side = OrderSide::with_capacity(Side::Buy, 10);
        side.append(new_order_id(), 100, 10);
        side.append(new_order_id(), 200, 20);

        let output = format!("{}", side);
        assert!(output.contains("10 -> 100"));
        assert!(output.contains("20 -> 200"));
    }

    #[test]
    fn test_display_sell() {
        let mut side = OrderSide::with_capacity(Side::Sell, 10);
        side.append(new_order_id(), 50, 5);
        side.append(new_order_id(), 150, 15);

        let output = format!("{}", side);
        // Per Side::Sell l'iteratore viene invertito
        assert!(output.contains("15 -> 150"));
        assert!(output.contains("5 -> 50"));
    }

    #[test]
    fn test_display_empty() {
        let side = OrderSide::with_capacity(Side::Buy, 10);
        let output = format!("{}", side);
        assert!(output.is_empty());
    }

    #[test]
    fn stress_test_random() {
        let mut os = OrderSide::with_capacity(Side::Buy, 10);
        let mut rng = rng();
        let mut orders = Vec::new();

        // aggiungo 1000 ordini su prezzi casuali tra 100..200
        for _ in 0..1000 {
            let id = new_order_id();
            let qty = rand::random::<u64>() % 500 + 1;
            let price = 100 + rand::random::<u64>() % 100;
            os.append(id, qty, price);
            orders.push((id, qty, price));
        }

        // aggiorno metà degli ordini con quantità casuali
        let mut to_update = orders.clone();
        to_update.shuffle(&mut rng);
        for (id, _, price) in to_update.iter().take(500) {
            let queue = os.take_queue(*price);
            let mut queue = queue.unwrap();
            let new_qty = rand::random::<u64>() % 500 + 1;
            queue.update(*id, 0, new_qty); // aggiorna quantità

            os.put_queue(*price, queue);
        }

        // rimuovo tutti gli ordini in ordine casuale
        orders.shuffle(&mut rng);
        for (id, qty, price) in orders.iter() {
            let queue = os.take_queue(*price);
            let mut queue = queue.unwrap();
            os.remove(*id, *qty, *price, &mut queue);

            if !queue.is_empty() {
                os.put_queue(*price, queue);
            }
        }

        assert!(os.is_empty());
        assert_eq!(os.volume, 0);
    }
}
