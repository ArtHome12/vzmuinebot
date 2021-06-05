/* ===============================================================================
Restaurant menu bot.
Database. 28 May 2020.
----------------------------------------------------------------------------
Licensed under the terms of the GPL version 3.
http://www.gnu.org/licenses/gpl-3.0.html
Copyright (c) 2020 by Artem Khomenko _mag12@yahoo.com.
=============================================================================== */

use std::collections::HashMap;
use once_cell::sync::{OnceCell};
use deadpool_postgres::{Pool, Client, };
use postgres_native_tls::MakeTlsConnector;
use tokio_postgres::{types::ToSql, Row, };
use async_recursion::async_recursion;


use crate::environment as env;
use crate::node::*;
use crate::customer::*;
use crate::orders::*;

// Пул клиентов БД
pub type PoolAlias = Pool<MakeTlsConnector>;
pub static DB: OnceCell<PoolAlias> = OnceCell::new();

pub type Params<'a> = &'a[&'a(dyn ToSql + Sync)];

// ============================================================================
// [Nodes table]
// ============================================================================

pub enum LoadNode {
   Owner(i64), // load first node with this owner
   Id(i32), // load node with specified id
   EnabledId(i32), // like Id but without disabled
   EnabledIdNoChildren(i32), // like EnabledId but without children
   EnabledNowId(i32), // like EnabledId but opened now
   Children(Node), // load children nodes for this
   EnabledChildren(Node), // like Children but without disabled
   EnabledChildrenNow(Node), // like EnabledChildren but opened now
}

#[async_recursion]
pub async fn node(mode: LoadNode) -> Result<Option<Node>, String> {
   async fn do_load_node(rows: Vec<Row>) -> Result<Option<Node>, String> {
      if rows.is_empty() {Ok(None)}
      else {
         // Create node
         let mut start_node =  Node::from(&rows[0]);

         // Try to find picture if not
         if start_node.picture.is_none() {
            start_node.picture = lookup_picture(start_node.parent).await?;
         }

         Ok(Some(start_node))
      }
   }

   async fn do_load_children(mode: LoadNode) -> Result<Option<Node>, String> {
      // Recursively load its children
      let with_children = node(mode)
      .await?
      .unwrap();
      Ok(Some(with_children))
   }

   fn convert_kind(kind: LoadNode, arg: Node) -> LoadNode {
      match kind {
         LoadNode::Id(_) | LoadNode::Owner(_) => LoadNode::Children(arg),
         LoadNode::EnabledId(_) => LoadNode::EnabledChildren(arg),
         LoadNode::EnabledNowId(_) => LoadNode::EnabledChildrenNow(arg),
         _ => panic!("convert_kind"),
      }
   }

   // DB client from the pool
   let client = db_client().await?;

   // Construct statement from parts
   let part_select = "SELECT id, parent, title, descr, picture, enabled, banned, owner1, owner2, owner3, open, close, price FROM nodes WHERE";
   let part_owner = "owner1 = $1::BIGINT OR owner2 = $1::BIGINT OR owner3 = $1::BIGINT";
   let part_id = "id = $1::BIGINT";
   let part_enabled = "AND enabled = TRUE AND banned = FALSE";
   let part_now = "AND (($2::TIME BETWEEN open AND close) OR (open >= close AND $2::TIME > open))";
   let part_children = "parent = $1::BIGINT";

   let where_tuple = match &mode {
      LoadNode::Owner(user_id) =>  (part_owner.to_string(), *user_id),
      LoadNode::Id(id) => (part_id.to_string(), *id as i64),
      LoadNode::EnabledId(id)
      | LoadNode::EnabledIdNoChildren(id) => (format!("{} {}", part_id, part_enabled), *id as i64),
      LoadNode::EnabledNowId(id) => (format!("{} {} {}", part_id, part_enabled, part_now), *id as i64),
      LoadNode::Children(node) => (part_children.to_string(), node.id as i64),
      LoadNode::EnabledChildren(node) => (format!("{} {}", part_children, part_enabled), node.id as i64),
      LoadNode::EnabledChildrenNow(node) => (format!("{} {} {}", part_children, part_enabled, part_now), node.id as i64),
   };

   let statement_text = format!("{} {}  ORDER BY id", part_select, where_tuple.0);

   // Prepare query
   let statement = client
   .prepare(&statement_text)
   .await
   .map_err(|err| format!("node prepare: {}", err))?;

   // Run query
   let query = match &mode {
      LoadNode::EnabledNowId(_)
      | &LoadNode::EnabledChildrenNow(_) => {
         // Current local time
         let time = env::current_date_time().time();
         client.query(&statement, &[&where_tuple.1, &time]).await
      }
      _ => client.query(&statement, &[&where_tuple.1]).await
   }
   .map_err(|err| format!("node query: {}", err))?;

   // Collect results
   match mode {

      LoadNode::Children(mut node)
      | LoadNode::EnabledChildren(mut node)
      | LoadNode::EnabledChildrenNow(mut node) => {
         // Clear any old and add new children
         node.children.clear();
         for row in query {
            // Create and inherit the picture if none
            let mut child = Node::from(&row);
            if child.picture.is_none() {
               child.picture = node.picture.clone();
            }

            // env::log(&format!("added {} id={}", child.title, child.id)).await;
            node.children.push(child);
         }

         Ok(Some(node))
      }

      LoadNode::Id(_)
      | LoadNode::Owner(_)
      | LoadNode::EnabledId(_)
      | LoadNode::EnabledNowId(_) => {

         // Create new node and initialize it from database
         let mut res = do_load_node(query).await?;
         if res.is_some() {
            let child_mode = convert_kind(mode, res.unwrap());
            res = do_load_children(child_mode).await?;
         }
         Ok(res)
      }

      LoadNode::EnabledIdNoChildren(_) => {
         let res = do_load_node(query).await?;
         Ok(res)
      }
   }
}

async fn lookup_picture(node_id: i32) -> Result<Option<String>, String> {
   let query = "WITH RECURSIVE cte AS (
         SELECT id, parent, picture FROM nodes WHERE id = $1::INTEGER
         UNION SELECT n.id, n.parent, n.picture FROM nodes n
         INNER JOIN cte ON cte.parent = n.id
      ) SELECT picture FROM cte WHERE picture IS NOT NULL LIMIT 1";

   // Prepare query
   let client = db_client().await?;
   let statement = client
   .prepare(&query)
   .await
   .map_err(|err| format!("lookup_picture prepare: {}", err))?;

   // Run query
   let query = client
   .query(&statement, &[&node_id])
   .await
   .map_err(|err| format!("lookup_picture query: {}", err))?;

   // Collect result
   let res = query.last()
   .map(|row| row.get(0));

   Ok(res)
}

pub async fn insert_node(node: &Node) -> Result<(), String> {
   // Information for query
   let sql_text = "INSERT INTO nodes (parent, title, descr, picture, enabled, banned, owner1, owner2, owner3, open, close, price) \
      VALUES ($1::INTEGER, $2::VARCHAR, $3::VARCHAR, $4::VARCHAR, $5::BOOLEAN, $6::BOOLEAN, $7::BIGINT, $8::BIGINT, $9::BIGINT, $10::TIME, $11::TIME, $12::INTEGER)";

   let i32_price = node.price as i32;
   let params: Params = &[&node.parent,
      &node.title,
      &node.descr,
      &node.picture,
      &node.enabled,
      &node.banned,
      &node.owners[0],
      &node.owners[1],
      &node.owners[2],
      &node.time.0,
      &node.time.1,
      &i32_price];

   // Run query
   execute_prepared_one(sql_text, params).await
}

pub async fn delete_node(id: i32) -> Result<(), String> {
   let client = db_client().await?;

   // Check no children
   let text = "SELECT id FROM nodes WHERE parent = $1::INTEGER";
   let query = client.query(text, &[&id])
   .await
   .map_err(|err| format!("delete_node prepare: {}", err))?;

   let children_num = query.len();
   if children_num > 0 {
      return Err(format!("delete_node has {} children", children_num));
   }

   // Delete orders of node
   let text = "DELETE FROM orders WHERE node_id = $1::INTEGER";
   execute_one(text, &[&id]).await?;

   // Delete node
   let text = "DELETE FROM nodes WHERE id = $1::INTEGER";
   execute_one(text, &[&id]).await
}

pub async fn update_node(id: i32, update: &UpdateNode) -> Result<(), String> {
   match &update.kind {
      UpdateKind::Text(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::VARCHAR WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
      UpdateKind::Picture(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::VARCHAR WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
      UpdateKind::Flag(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::BOOLEAN WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
      UpdateKind::Int(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::BIGINT WHERE id=$2::INTEGER", update.field);
         execute_one(text.as_str(), &[new_val, &id]).await
      }
      UpdateKind::Time(open, close) => {
         let text = "UPDATE nodes SET open = $1::TIME, close = $2::TIME WHERE id=$3::INTEGER";
         execute_one(text, &[open, close, &id]).await
      }
      UpdateKind::Money(new_val) => {
         let text = format!("UPDATE nodes SET {} = $1::INTEGER WHERE id=$2::INTEGER", update.field);
         let i32_new_val = *new_val as i32;
         execute_one(text.as_str(), &[&i32_new_val, &id]).await
      }
   }
}

// ============================================================================
// [Users]
// ============================================================================
// Update last seen field or return false if user don't exist
pub async fn user_update_last_seen(user_id: i64) -> Result<bool, String> {
   let query = "UPDATE users SET last_seen = NOW() WHERE user_id=$1::BIGINT";

   // Prepare query
   let client = db_client().await?;
   let statement = client
   .prepare(&query)
   .await
   .map_err(|err| format!("user_update_last_seen prepare: {}", err))?;

   // Run query
   let query = client
   .execute(&statement, &[&user_id])
   .await
   .map_err(|err| format!("user_update_last_seen execute: {}", err))?;

   // Return result
   Ok(query == 1)
}

// Store new user
pub async fn user_insert(id: i64, name: String, contact: String) -> Result<(), String> {
   let sql_text = "INSERT INTO users (user_id, user_name, contact, address, last_seen, pickup) VALUES ($1::INTEGER, $2::VARCHAR, $3::VARCHAR, '-', NOW(), FALSE)";
   execute_one(sql_text, &[&id, &name, &contact]).await?;

   // Notify about a new user
   env::log(&format!("Новый пользователь id={}, {}, {}", id, name, contact)).await;
   Ok(())
}

pub async fn user(user_id: i64) -> Result<Customer, String> {
   let sql_text = "SELECT user_name, contact, address, pickup FROM users WHERE user_id=$1::BIGINT";

   // Prepare query
   let client = db_client().await?;
   let statement = client
   .prepare(&sql_text)
   .await
   .map_err(|err| format!("user prepare: {}", err))?;

   // Run query
   let query = client
   .query(&statement, &[&user_id])
   .await
   .map_err(|err| format!("user query: {}", err))?;

   // Return result
   let len = query.len();
   if len == 1 {
      let row = &query[0];

      let delivery = if row.get(3) { Delivery::Pickup } else { Delivery::Courier };

      let res = Customer {
         name: row.get(0),
         contact: row.get(1),
         address:  row.get(2),
         delivery,
      };

      Ok(res)
   } else {
      Err(format!("user query return: {} recs instead one", len))
   }
}

async fn update_user_str(id: i64, new_val: &String, field: &str) -> Result<(), String> {
   let text = format!("UPDATE users SET {} = $1::VARCHAR WHERE user_id=$2::BIGINT", field);
   execute_one(text.as_str(), &[new_val, &id]).await
}

pub async fn update_user_name(id: i64, name: &String) -> Result<(), String> {
   update_user_str(id, name, "user_name").await
}

pub async fn update_user_contact(id: i64, contact: &String) -> Result<(), String> {
   update_user_str(id, contact, "contact").await
}

pub async fn update_user_address(id: i64, address: &String) -> Result<(), String> {
   update_user_str(id, address, "address").await
}

pub async fn update_user_delivery(id: i64, delivery: &Delivery) -> Result<(), String> {
   let text = "UPDATE users SET pickup = $1::BOOLEAN WHERE user_id=$2::BIGINT";
   let new_val = matches!(delivery, Delivery::Pickup);
   execute_one(text, &[&new_val, &id]).await
}

// ============================================================================
// [Orders]
// ============================================================================
pub async fn amount(user_id: i64, node_id: i32) -> Result<usize, String> {
   let query = "SELECT amount FROM orders WHERE user_id=$1::BIGINT AND node_id=$2::INTEGER";

   // Prepare query
   let client = db_client().await?;
   let statement = client
   .prepare(&query)
   .await
   .map_err(|err| format!("amount prepare: {}", err))?;

   // Run query
   let query = client
   .query(&statement, &[&user_id, &node_id])
   .await
   .map_err(|err| format!("amount query: {}", err))?;

   // Return result
   let res = if query.is_empty() { 0usize }
   else {
      let res: i32 = query[0].get(0);
      res as usize
   };
   Ok(res)
}

pub async fn amount_inc(user_id: i64, node_id: i32) -> Result<(), String> {
   let query = "INSERT INTO orders as o (user_id, node_id, amount) VALUES ($1::BIGINT, $2::INTEGER, 1)
      ON CONFLICT ON CONSTRAINT orders_pkey DO
      UPDATE SET amount = o.amount + 1 WHERE o.user_id = $1::BIGINT AND o.node_id = $2::INTEGER";

   // Prepare query
   let client = db_client().await?;
   let statement = client
   .prepare(&query)
   .await
   .map_err(|err| format!("amount_add prepare: {}", err))?;

   // Run query
   let query = client
   .execute(&statement, &[&user_id, &node_id])
   .await
   .map_err(|err| format!("amount_add execute: {}", err))?;

   // Return result
   if query != 1 {
      Err(format!("amount_add execute user_id={}, node_id={} return {} recs instead one", user_id, node_id, query))
   } else { Ok(()) }
}

pub async fn amount_dec(user_id: i64, node_id: i32) -> Result<(), String> {
   let query = "UPDATE orders SET amount = amount - 1 WHERE user_id = $1::BIGINT AND node_id = $2::INTEGER";

   // Prepare query
   let client = db_client().await?;
   let statement = client
   .prepare(&query)
   .await
   .map_err(|err| format!("amount_dec prepare: {}", err))?;

   // Run query
   let query = client
   .execute(&statement, &[&user_id, &node_id])
   .await
   .map_err(|err| format!("amount_dec execute: {}", err))?;

   // Return result
   if query != 1 {
      Err(format!("amount_dec execute user_id={}, node_id={} return {} recs instead one", user_id, node_id, query))
   } else { Ok(()) }
}

pub async fn orders(user_id: i64) -> Result<Orders, String> {
   struct Order {
      node_id: i32,
      amount: i32,
   }
   async fn do_load_orders(user_id: i64) -> Result<Vec<Order>, String> {
      let query = "SELECT node_id, amount FROM orders WHERE user_id = $1::BIGINT AND amount > 0";

      // Prepare query
      let client = db_client().await?;
      let statement = client
      .prepare(&query)
      .await
      .map_err(|err| format!("orders prepare: {}", err))?;

      // Run query
      let query = client
      .query(&statement, &[&user_id])
      .await
      .map_err(|err| format!("orders query: {}", err))?;

      // Return result
      let res = query.iter()
      .map(|row| Order {
         node_id: row.get(0),
         amount: row.get(1),
      }).collect();
      Ok(res)
   }

   async fn do_lookup_owner(start_id: i32) -> Result<i32, String> {
      let query = "WITH RECURSIVE cte AS (
         SELECT id, parent, owner1 FROM nodes WHERE id = $1::INTEGER
         UNION SELECT n.id, n.parent, n.owner1 FROM nodes n
         INNER JOIN cte ON cte.parent = n.id
      ) SELECT id FROM cte WHERE owner1 > 0 LIMIT 1";

      // Prepare query
      let client = db_client().await?;
      let statement = client
      .prepare(&query)
      .await
      .map_err(|err| format!("do_lookup_owner prepare: {}", err))?;

      // Run query
      let query = client
      .query(&statement, &[&start_id])
      .await
      .map_err(|err| format!("do_lookup_owner query: {}", err))?;

      // Collect result
      let res = query.last()
      .map(|row| row.get(0))
      .unwrap_or_default();

      Ok(res)
   }

   // Load node ids and amounts
   let id_amount_pair = do_load_orders(user_id).await?;

   // Load nodes and group its by owner
   let mut orders: HashMap<i32, Vec<NodeWithAmount>> = HashMap::new();
   for item in id_amount_pair {
      let node = node(LoadNode::EnabledIdNoChildren(item.node_id)).await?;

      if let Some(node) = node {

         // Lookup for owner (for grouping later)
         let node_with_owner = if node.owners[0] > 0 { node.id }
         else { do_lookup_owner(node.parent).await? };

         let node = NodeWithAmount{
            amount: item.amount as usize,
            node,
         };

         // Add to existing owner or to the new
         let owner = orders.get_mut(&node_with_owner);
         match owner {
            Some(owner) => owner.push(node),
            None => { orders.insert(node_with_owner, vec![node]); },
         }
      }
   }

   // Load owners node for contact info
   let mut res = Orders::new();
   for order in orders {
      let owner = node(LoadNode::EnabledIdNoChildren(order.0)).await?;
      if let Some(owner) = owner {
         res.data.insert(owner, order.1);
      }
   }

   Ok(res)
}

pub async fn delete_order(user_id: i64, node_id: i32) -> Result<(), String> {
   let text = "DELETE FROM orders WHERE user_id = $1::BIGINT AND node_id = $2::INTEGER";
   execute_one(text, &[&user_id, &node_id]).await
}

pub async fn delete_orders(user_id: i64) -> Result<(), String> {
   let text = "DELETE FROM orders WHERE user_id = $1::BIGINT OR amount < 1";
   execute_prepared(text, &[&user_id]).await?;
   Ok(())
}


// ============================================================================
// [Misc]
// ============================================================================

pub async fn is_tables_exist() -> bool {
   // DB client from the pool
   let client = db_client().await;
   if client.is_err() {return false;}

   // Check that one of tables exists
   let rows = client.unwrap()
   .query("SELECT table_name FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_NAME='nodes'", &[]).await;

   // Return result
   match rows {
      Ok(data) => !data.is_empty(),
      _ => false,
   }
}

pub async fn create_tables() -> bool {
   let client = db_client().await;
   if client.is_err() {return false;}

   let query = client.unwrap()
   .batch_execute("CREATE TABLE nodes (
         PRIMARY KEY (id),
         id             SERIAL         NOT NULL,
         parent         INTEGER        NOT NULL,
         title          VARCHAR        NOT NULL,
         descr          VARCHAR        NOT NULL,
         picture        VARCHAR,
         enabled        BOOLEAN        NOT NULL,
         banned         BOOLEAN        NOT NULL,
         owner1         BIGINT         NOT NULL,
         owner2         BIGINT         NOT NULL,
         owner3         BIGINT         NOT NULL,
         open           TIME           NOT NULL,
         close          TIME           NOT NULL,
         price          INTEGER        NOT NULL);

         INSERT INTO nodes (id, parent, title, descr, picture, enabled, banned, owner1, owner2, owner3, open, close, price)
         VALUES (0, -1, 'Добро пожаловать', '-', '', true, false, 0, 0, 0, '00:00', '00:00', 0);

         CREATE TABLE users (
            PRIMARY KEY (user_id),
            user_id        INTEGER        NOT NULL,
            user_name      VARCHAR(100)   NOT NULL,
            contact        VARCHAR(100)   NOT NULL,
            address        VARCHAR(100)   NOT NULL,
            last_seen      TIMESTAMP      NOT NULL,
            pickup         BOOLEAN        NOT NULL);

         CREATE TABLE orders (
            PRIMARY KEY (user_id, node_id),
            user_id        BIGINT         NOT NULL,
            node_id        INTEGER        NOT NULL,
            amount         INTEGER        NOT NULL);
   ")
   .await;

   match query {
      Ok(_) => true,
      Err(e) => {
         env::log(&format!("Error create_tables: {}", e)).await;
         false
       }
   }
}

// Convert bool to text
pub fn is_success(flag : bool) -> &'static str {
   if flag {
      "успешно"
  } else {
      "ошибка"
  }
}

// Обёртка, возвращает пул клиентов
async fn db_client() -> Result<Client::<MakeTlsConnector>, String> {
   match DB.get().unwrap().get().await {
      Ok(client) => Ok(client),
      Err(e) => {
         let error = format!("No db client: {}", e);
         env::log(&error).await;
         Err(error)
      }
   }
}

async fn execute_one(sql_text: &str, params: &[&(dyn ToSql + Sync)]) -> Result<(), String> {
   // DB client from the pool
   let client = db_client().await?;

   // Run query
   let query = client.execute(sql_text, params)
   .await
   .map_err(|err| format!("execute_one {} execute: {}", sql_text, err))?;

   // Only one records has to be affected
   if query == 1 { Ok(()) }
   else { Err(format!("execute_one {}: affected {} records instead one (params: {:?})", sql_text, query, params)) }
}

async fn execute_prepared(sql_text: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, String> {
   // DB client from the pool
   let client = db_client().await?;

   // Prepare query
   let statement = client
   .prepare(&sql_text)
   .await
   .map_err(|err| format!("execute_prepared {} prepare: {}", sql_text, err))?;

   // Run query
   let query = client.execute(&statement, params)
   .await
   .map_err(|err| format!("execute_prepared {} prepare: {}", sql_text, err))?;

   Ok(query)
}

async fn execute_prepared_one(sql_text: &str, params: &[&(dyn ToSql + Sync)]) -> Result<(), String> {
   // Only one record has to be updated
   let updated = execute_prepared(sql_text, params).await?;
   if updated == 1 { Ok(()) }
   else { Err(format!("execute_prepare_one {} updated {} records", sql_text, updated)) }
}

