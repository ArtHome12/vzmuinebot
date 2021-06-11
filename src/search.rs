/* ===============================================================================
Restaurant menu bot.
Search algorithms. 11 Jun 2021.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use crate::database as db;

pub struct IdTilePair {
   pub id: i32, // node id
   pub title: String, // node title
}

pub type IdTilePairs = Vec<IdTilePair>;

pub async fn search(pattern: &String) -> Result<String, String> {
   let raw = db::node_search(pattern).await?;

   Ok(String::from("ничего не найдено"))
}