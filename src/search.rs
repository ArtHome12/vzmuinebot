/* ===============================================================================
Restaurant menu bot.
Search algorithms. 11 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020-2022 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use crate::database as db;
use crate::general;

pub struct IdTilePair {
   pub id: i32, // node id
   pub title: String, // node title
}

// Nodes from found to root
pub type Chain = Vec<IdTilePair>;

pub struct Search {
   chains: Vec<Chain>,
}

impl Search {

   pub fn new(chains: Vec<Chain>) -> Self {
      Self {
         chains,
      }
   }

   // Returns true if there were changes
   fn cut_common_root(&mut self) -> bool {
      // Do not cut single record
      if self.chains.len() <= 1 {
         return false;
      }

      // Check on the coincidence
      let mut it = self.chains.iter();

      // Unwrap checked above and there should be no empty elements
      let pattern_id = it.next().unwrap().last().unwrap().id;

      let equal = it.all(|f|
         f.last().and_then(|f| (f.id == pattern_id).then(|| ())).is_some()
      );
      
      // Throw out the coinciding part
      if equal {
         self.chains.iter_mut()
         .for_each(|f| {
            f.pop();
         });
   
         // Next iteration
         self.cut_common_root()
      } else {
         false
      }
   }
}


pub async fn search(pattern: &str) -> Result<Vec<String>, String> {

   fn chain_to_str(chain: &Chain) -> String {
      if chain.is_empty() {
         String::default()
      } else {
         let id = chain.first().unwrap().id;
         let init = format!(" {}{}", general::Command::Goto(0).as_ref(), id);
         chain.iter().fold(init, |acc, f| format!("{}{}", String::from("/") + &f.title, acc))
      }
   }

   // main body

   // Redundant data from the database
   let mut sep = db::node_search(pattern).await?;

   // Cut the coincident root
   sep.cut_common_root();

   // Map chains into strings
   let res = sep.chains.iter()
   .map(|v| chain_to_str(v))
   .collect();

   Ok(res)
}