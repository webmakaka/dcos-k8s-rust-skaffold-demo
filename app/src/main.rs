#[macro_use] extern crate diesel;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;

extern crate rocket;
extern crate dotenv;

mod api;
mod errors;
mod forms;
mod models;
mod postgres;
mod schema;

use api::gen_routes;
// use api::{gen_routes, gen_errors};

#[rocket::main]
async fn main() {
    let _ = rocket::build()
    .mount("/", gen_routes())
    // .register("/", catchers![
    //     gen_errors()
    // ])
    .launch()
    .await;
}
