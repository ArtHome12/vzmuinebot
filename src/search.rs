/* ===============================================================================
Restaurant menu bot.
Search algorithms. 11 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use crate::database as db;
use crate::general;

pub struct IdTilePair {
   pub id: i32, // node id
   pub title: String, // node title
}

// List of found nodes
// pub type IdTitlePairs = Vec<IdTilePair>;

// Nodes from found to root
pub type Chain = Vec<IdTilePair>;

struct Separator {
   chains: Vec<Chain>,
}

impl Separator {
   // Returns true if there were changes
   fn cut_common_root(&mut self) -> bool {
      // Check on the coincidence
      let mut it = self.chains.iter();
      if let Some(first) = it.next() {
         let id = if !first.is_empty() { first.last().unwrap().id }
         else { return false; };
         
         it.all(|f|
            f.last().and_then(|f| (f.id == id).then(|| ())).is_some()
         )
      } else { return false };
      
      // Throw out the coinciding part
      self.chains.iter_mut()
      .for_each(|f| {
         f.pop();
      });

      // Next iteration
      self.cut_common_root()
   }
}


pub async fn search(pattern: &String) -> Result<String, String> {

   fn chain_to_str(chain: &Chain) -> String {
      chain.iter().fold(format!(" {}{}", general::Command::Goto(0).as_ref(), 0), |acc, f| format!("{}{}", String::from("/") + &f.title, acc))
   }

   // Redundant data from the database
   let raw = db::node_search(pattern).await?;
   let mut sep = Separator { 
      chains: raw,
   };

   let res = if sep.chains.is_empty() {
      String::from("Ничего не найдено")
   } else {
      // Cut the coincident root
      // sep.cut_common_root();

      sep.chains.iter()
      .fold(String::default(), |acc, v| {
         format!("{}\n{}", acc, chain_to_str(v))
      })
   };

   Ok(res)
}