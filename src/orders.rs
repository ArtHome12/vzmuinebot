/* ===============================================================================
Restaurant menu bot.
A set of items (nodes), grouped by owners for forming an order. 03 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use std::collections::HashMap;

use crate::node::*;

pub struct NodeWithAmount {
   pub amount: i32,
   pub node: Node,
}

pub type Order = Vec<NodeWithAmount>;


pub struct Orders {
   pub data: HashMap<Node, Order>,
}

impl Orders {
   pub fn new() -> Self {
      Self {
         data: HashMap::new(),
      }
   }

   /*pub fn add(&mut self, owner: Node, order: NodeWithAmount) {
      // Group nodes by owner
      let owner_orders = self.data.get_mut(&owner);
      match owner_orders {
         Some(owner_order) => owner_order.push(order),
         None => { self.data.insert(owner, vec![order]); },
      }
   }

   pub fn announce(&self) -> String {
      if self.data.is_empty() {
         String::from("Корзина пуста")
      } else {
         format!("В корзине {} поз. на общую сумму 0", 0)
      }
   }

   pub fn descr(&self, owner: i64) -> String {
      format!("owner={} with {} items", owner, 0)
   }

   pub fn owners(&self) -> Vec<i64> {
      self.data
      .keys()
      .map(|v| *v)
      .collect()
   } */
}