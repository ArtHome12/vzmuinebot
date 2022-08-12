/* ===============================================================================
Restaurant menu bot.
A set of items (nodes), grouped by owners for forming an order. 03 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use std::collections::HashMap;

use crate::node::*;

pub struct NodeWithAmount {
   pub amount: usize,
   pub node: Node,
}

impl NodeWithAmount {
    pub fn cost(&self) -> usize {
       self.amount * self.node.price
    }
}

pub type Order = Vec<NodeWithAmount>;

pub struct CartInfo {
   pub orders_num: usize,
   pub items_num: usize,
   pub total_cost: usize,
}

pub struct Orders {
   pub data: HashMap<Node, Order>,
}

impl Orders {
   pub fn new() -> Self {
      Self {
         data: HashMap::new(),
      }
   }

   pub fn cart_info(&self) -> CartInfo {
      let mut res = CartInfo {
         orders_num: 0,
         items_num: 0,
         total_cost: 0
      };

      for owner in &self.data {
         let (o, i, t) = owner.1.iter()
         .fold((0, 0, 0), |acc, v| {
            (acc.0 + 1, acc.1 + v.amount, acc.2 + v.cost())
         });

         res.orders_num += o;
         res.items_num += i;
         res.total_cost += t;
      };

      res
   }
}