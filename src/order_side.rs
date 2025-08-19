use std::fmt;
use std::cmp::Ordering;
use std::collections::{HashMap};
use rb_tree::RBQueue;
use uuid::Uuid;

use crate::math::math::{safe_add, safe_sub};
use crate::order_queue::OrderQueue;
use crate::{Side};

#[derive(Debug)]
pub struct OrderSide {
    pub prices_tree: RBQueue<u128, Box<dyn Fn(&u128, &u128) -> Ordering>>,
    pub prices: HashMap<u128, OrderQueue>,
    pub volume: u128,
    pub side: Side,
}

impl OrderSide {
    pub fn new(side: Side) -> OrderSide {
        let side_for_cmp = side;
        let comparator = move |a: &u128, b: &u128| {
            match side_for_cmp {
                Side::Sell => a.cmp(b),  // ordine crescente
                Side::Buy => b.cmp(a),   // ordine decrescente
            }
        };
        OrderSide { 
            side,
            prices: HashMap::new(),
            prices_tree: RBQueue::new(Box::new(comparator)),
            volume: 0,
        }
    }

    // appends order to definite price level
    pub fn append (&mut self, id: Uuid, quantity: u128, price: u128) {
        let queue = self.prices.entry(price).or_insert_with(|| {
            self.prices_tree.insert(price);
            OrderQueue::new(price)
        });

		self.volume = safe_add(self.volume, quantity);        
        queue.append(id, quantity);
    }

    // removes order from definite price level
	pub fn remove (&mut self, id: Uuid, quantity: u128, price: u128, queue: &mut OrderQueue) {
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

    pub fn take_queue(&mut self, price: u128) -> Option<OrderQueue> {
        self.prices.remove(&price)
    }

    pub fn put_queue(&mut self, price: u128, q: OrderQueue) {
        self.prices.insert(price, q);
    }

    pub fn best_price(&self, min: bool) -> Option<u128> {
        let price = match (self.side, min) {
            (Side::Sell, true) | (Side::Buy, false) => self.prices_tree.peek(),
            (Side::Sell, false) | (Side::Buy, true) => self.prices_tree.peek_back(),
        };
        price.copied()
    }

    // returns max level of price
    pub fn min_price(&self) -> Option<u128> {
        self.best_price(true)
    }

    // returns min level of price
    pub fn max_price(&self) -> Option<u128> {
        self.best_price(false)
    }

    pub fn depth(&self, limit: u32) -> Vec<(u128, u128)> {
        let mut depth = Vec::new();
        let mut count = 0;

        for price in self.prices_tree.ordered() {
            if count >= limit {
                break;
            }
            if let Some(queue) = self.prices.get(price) {
                depth.push((*price, queue.volume));
                count += 1;
            }
        }

        depth
    }
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prices = self.prices_tree.ordered();
        let iter: Box<dyn Iterator<Item = &&u128>> = match self.side {
            Side::Sell => Box::new(prices.iter().rev()),
            Side::Buy => Box::new(prices.iter()),
        };

        for price in iter {
            if let Some(queue) = self.prices.get(&price) {
                writeln!(f, "{} -> {}", price, queue.volume)?;
            }
        }

        Ok(())
    }
}