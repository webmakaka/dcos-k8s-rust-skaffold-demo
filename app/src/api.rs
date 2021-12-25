use diesel::{delete, insert_into, update};
use diesel::prelude::*;

use rocket;
use rocket::{Catcher, Route, Request};
use rocket::response::status::{BadRequest, Created, NoContent};
use rocket::serde::json::Json;

use crate::errors::ApiError;
use crate::forms::EmployeeForm;
use crate::models::{Employee, EmployeeList};
use crate::postgres::connect as dbc;

// -----------------------------------------------------------------------------
// HTTP Errors
// -----------------------------------------------------------------------------

#[rocket::catch(404)]
fn not_found(_: &Request) -> Json<ApiError> {
    Json(ApiError{
        message: "not found".to_string(),
    })
}

// -----------------------------------------------------------------------------
// HTTP GET, PUT, POST & DELETE
// -----------------------------------------------------------------------------

#[rocket::get("/employees", format = "application/json")]
fn employee_list() -> Json<EmployeeList> {
    use super::schema::employees::dsl::*;

    let db = dbc();
    let results = employees.load::<Employee>(&db)
        .expect("Error loading Employees");

    Json(EmployeeList {
        results: results.to_vec(),
    })
}

#[rocket::get("/employees/<employee_id>", format = "application/json")]
fn employee_get(employee_id: i32) -> Option<Json<Employee>> {
    use super::schema::employees::dsl::*;

    let db = dbc();
    match employees.find(employee_id).first::<Employee>(&db) {
        Ok(employee) => Some(Json(employee)),
        Err(_) => None,
    }
}

#[rocket::put("/employees", format = "application/json", data = "<json_employee>")]
fn employee_put(json_employee: Json<EmployeeForm>) -> Result<Created<()>, BadRequest<String>> {
    use super::schema::employees::dsl::*;

    let mut new_employee = json_employee.into_inner();
    new_employee.id = None;
    let insert = insert_into(employees)
        .values(&new_employee);

    let db = dbc();
    match insert.execute(&db) {
        Ok(_) => {
            Ok(Created("/employees".to_string(), Some(())))
        },
        Err(err) => {
            let err = json!({"error": err.to_string()});
            Err(BadRequest(Some(err.to_string())))
        },
    }
}

#[rocket::post("/employees/<employee_id>", format = "application/json", data = "<json_employee>")]
fn employee_update(employee_id: i32, json_employee: Json<EmployeeForm>) -> Result<NoContent, BadRequest<String>> {
    use super::schema::employees::dsl::*;

    let employee = json_employee.into_inner();
    let update = update(employees.filter(id.eq(employee_id)))
        .set(&employee);

    let db = dbc();
    match update.execute(&db) {
        Ok(_) => Ok(NoContent),
        Err(err) => {
            let err = json!({"error": err.to_string()});
            Err(BadRequest(Some(err.to_string())))
        },
    }
}

#[rocket::delete("/employees/<employee_id>", format = "application/json")]
fn employee_delete(employee_id: i32) -> Option<NoContent> {
    use super::schema::employees::dsl::*;

    let db = dbc();
    let deleted = delete(employees.find(employee_id)).execute(&db)
        .expect("Error deleting Employee");

    if deleted >= 1 {
        Some(NoContent)
    } else {
        None
    }
}

// -----------------------------------------------------------------------------
// HTTP Routes
// -----------------------------------------------------------------------------

pub fn gen_routes() -> Vec<Route> {
    rocket::routes![employee_list, employee_get, employee_put, employee_update, employee_delete]
    // rocket::routes![employee_list, employee_get, employee_update, employee_delete]
}

pub fn gen_errors() -> Vec<Catcher> {
    rocket::catchers![not_found]
}
