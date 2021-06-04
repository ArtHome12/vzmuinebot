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
   pub data: HashMap<i64, Order>,
}

impl Orders {
   pub fn new() -> Self {
      Self {
         data: HashMap::new(),
      }
   }

   pub fn add(&mut self, node: NodeWithAmount) {
      // Group nodes by owner
      let owner = node.node.owners[0];
      let order = self.data.get_mut(&owner);
      match order {
         Some(order) => order.push(node),
         None => { self.data.insert(owner, vec![node]); },
      }
   }

   pub fn announce(&self) -> String {
      if self.data.is_empty() {
         String::from("Корзина пуста")
      } else {
         format!("В корзине {} поз. на общую сумму 0", 0);
         todo!()
      }
   }

   pub fn descr(&self, owner: i64) -> String {
      format!("owner={} with {} items", owner, self.data.get(&owner).unwrap().len())
   }

   pub fn owners(&self) -> Vec<i64> {
      self.data
      .keys()
      .map(|v| *v)
      .collect()
   }
}