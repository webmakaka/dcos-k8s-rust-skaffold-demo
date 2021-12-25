# Deploying Rust to Kubernetes on minikube with Skaffold

[YouTube conference recording](https://www.youtube.com/watch?v=9S1-69Rp1vQ)

<br/>

```
$ sudo apt-get install libpq-dev
```

<br/>

```
$ docker-compose up
```

<br/>

<!--

```
$ diesel setup --database-url=database.postgres
```
-->

```
$ cargo install diesel_cli --no-default-features --features postgres
$ diesel migration run
```

<br/>

```
$ cargo run
```

<br/>

```
$ curl http://127.0.0.1:8000/employees
```

<br/>

This is a demonstration project for using [Skaffold][0] to pipeline the development of your [Rust][1] web applications to Kubernetes on minikube.

## Overview

This demo is divided up into several different steps:

* Step 1 - Base Rust Application
* Step 2 - Deployment
* Step 3 - Database
* Step 4 - REST API
* Step 5 - Conclusions

## Requirements

* [minikube][2]
* [kubectl][5]
* [skaffold][0]
* [rust][1] using [rustup][47] (run `rustup default nightly-2018-09-17` to get the specific build for this demo)

# Step 1 - Base Rust App

In this step we're going to get our baseline application built.

We're going to use [rocket.rs][7], a web framework for Rust that makes it simple to write web apps fast.

## App Skeleton

Add the `Cargo.toml` file to add our Rust depedencies:

```toml
cat <<'EOF' > Cargo.toml
[package]
name = "rust-web-demo"
version = "0.1.0"

[dependencies]
diesel = { version = "1.3.3", features = ["postgres"] }
dotenv = "0.13.0"
rocket = "0.3.16"
rocket_codegen = "0.3.16"
serde = "1.0.79"
serde_json = "1.0.27"
serde_derive = "1.0.79"

[dependencies.rocket_contrib]
version = "0.3.16"
default-features = false
features = ["json"]
EOF
```

And then create the `src/main.rs` file with the following contents:

```rust
mkdir -p src/ && cat <<'EOF' > src/main.rs
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::config::{Config, Environment};

#[get("/")]
fn hello() -> String {
    format!("Rocket Webserver!")
}

fn main() {
    let config = Config::build(Environment::Staging)
        .address("0.0.0.0")
        .port(8000)
        .finalize()
        .unwrap();

    rocket::custom(config, true)
        .mount("/", routes![hello])
        .launch();
}
EOF
```

## Dockerfile

In the app directory, create your `Dockerfile` with the following contents:

```dockerfile
cat <<'EOF' > Dockerfile
FROM rustlang/rust@sha256:b62c21120fa9ef720e76f75fcdb53926ddace89feb2e21a1b5944554499aee86

RUN apt-get update

RUN apt-get install musl-tools -y

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/rust-web-demo

COPY Cargo.toml Cargo.toml

RUN mkdir src/

RUN echo "extern crate rocket;\nfn main() {println!(\"if you see this, the build broke\")}" > src/main.rs

RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

RUN rm -rf src/

RUN rm -f /usr/src/rust-web-demo/target/x86_64-unknown-linux-musl/release/rust-web-demo*

RUN rm -f /usr/src/rust-web-demo/target/x86_64-unknown-linux-musl/release/deps/rust_web_demo*

RUN rm -f /usr/src/rust-web-demo/target/x86_64-unknown-linux-musl/release/rust-web-demo.d

COPY src/* src/

RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl

FROM alpine:latest

RUN apk add --no-cache libpq

WORKDIR /root/

COPY --from=0 /usr/src/rust-web-demo/target/x86_64-unknown-linux-musl/release/rust-web-demo .

CMD ["./rust-web-demo"]
EOF
```

* **NOTE**: the first `FROM` here pulls a nightly version of Rust because [rocket.rs][7] requires nightly to build
* **NOTE**: as of writing this `cargo` [does not yet have a --dependencies-only build option][10] so there are some file removals and rebuilds used to improve Docker caching of builds
* **NOTE**: a [multi-stage docker build][29] is used for this Dockerfile to build a small (<20MB) image for this app (based on [alpine][30])

Also create a `.dockerignore` file to avoid adding files that aren't needed in the docker build:

```
cat <<'EOF' > .dockerignore
k8s/
target/
migrations/
skaffold.yaml
skaffold-deployment.yaml
Cargo.lock
LICENSE
EOF
```

Throughout this demo it's assumed you're going to [push][11] your [Docker Image][12] to [Docker Hub][13] so make sure you're logged in with [docker login][14].

## First Test

You can build the Docker image to test the above locally with:

```
docker build -t rust-web-demo .
docker run -p 8000:8000 -d --name rust-web-demo rust-web-demo
```

Access the demo by navigating to http://localhost:8000.

```
open http://localhost:8000
```

Clean up the container by running:

```
docker kill rust-web-demo
docker rm rust-web-demo
```

# Step 2 - Deployment with Skaffold

In this step we're going to start using [skaffold][0] to continuously ship updates to our code to our Kubernetes on DC/OS cluster.

First we'll simply deploy the app with a basic [Kubernetes Deployment][8], but then we'll update it by adding a [PostgreSQL][9] container and watch Skaffold ship our changes out to Kubernetes.

## Kubernetes Deployment Manifest

We will use a [Kubernetes Deployment][8] to run our application on the cluster via [Skaffold][0].

Create the manifest file `skaffold-deployment.yaml` with the following contents:

```yaml
cat <<'EOF' > skaffold-deployment.yaml
apiVersion: v1
kind: Service
metadata:
  name: rust-web-demo
spec:
  type: NodePort
  ports:
  - port: 8000
    protocol: TCP
  selector:
    app: rust-web-demo
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rust-web-demo
spec:
  replicas: 1
  selector:
    matchLabels:
      app: rust-web-demo
  template:
    metadata:
      labels:
        app: rust-web-demo
    spec:
      containers:
      - name: rust-web-demo
        image: docker.io/gkleiman/rust-web-demo
        ports:
        - containerPort: 8000
EOF
```

## Skaffold

See [installation][15] on the Skaffold Github repo, and install the right version for your system.

## Skaffold Configuration

To configure Skaffold we're going to create the file `skaffold.yaml` with the following contents:

```yaml
cat <<'EOF' > skaffold.yaml
apiVersion: skaffold/v1alpha5
kind: Config
build:
  artifacts:
  - image: docker.io/gkleiman/rust-web-demo
    context: .
    docker: {}
  local: {}
deploy:
  kubectl:
    manifests:
    - skaffold-deployment.yaml
EOF
```

## First Deployment

Now that we have the baseline application in place and Skaffold installed and configured we can start deploying.

Dedicate one terminal for running `skaffold` in the foreground and watching its logs.

In the terminal you selected for running `skaffold`, run the following:

```
skaffold dev
```

**NOTE**: You can optionally add `-v debug` option when running `skaffold dev` if you'd like to watch verbose information about what Skaffold is doing, or if you're having problems.

Your image will be built, pushed to Docker hub and deployed to your K8s cluster. The first build may take a long time as several artifacts need to be cached.

You can check on the status of your `rust-web-demo` deployment with `kubectl get deployment rust-web-demo`. Once it's complete, you should soon be able to access your app with:

```
curl `minikube service rust-web-demo --url`
```

Once everything is working, you'll receive the response `Rocket Webserver!`.

## Automatic Re-Deployment

With Skaffold now running and the first deployment of our services pushed up, any changes we make to code will result in a re-build and re-deployment.

Let's override the `src/main.rs` file so that the previous output "Rocket Webserver!" is replaced with "Skaffold updated me!":

```rust
cat <<'EOF' > src/main.rs
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

use rocket::config::{Config, Environment};

#[get("/")]
fn hello() -> String {
    format!("Skaffold updated me!")
}

fn main() {
    let config = Config::build(Environment::Staging)
        .address("0.0.0.0")
        .port(8000)
        .finalize().unwrap();

    rocket::custom(config, true)
        .mount("/", routes![hello])
        .launch();
}
EOF
```

After running the above and making the changes to `src/main.rs`, skaffold should build, push and deploy the changes to your cluster.

After the deployment finishes, test it with:

```
curl `minikube service rust-web-demo --url`
```

You should receive the response `Skaffold updated me!`.

## Deploy PostgreSQL

**WARNING**: The PostgreSQL deployment here is for demonstration purposes only. It's not HA, persistent, nor is it configured securely with SSL.

Now we'll configure a minimal [PostgreSQL][9] Database and add it to our Kubernetes deployment and let Skaffold ship the changes up.

Add a new deployment to `skaffold-deployment.yaml` for the PostgreSQL server:

```yaml
cat <<'EOF' >> skaffold-deployment.yaml
---
apiVersion: v1
kind: Service
metadata:
  name: rust-web-demo-postgres
spec:
  type: NodePort
  ports:
  - port: 5432
    protocol: TCP
  selector:
    app: rust-web-demo-postgres
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rust-web-demo-postgres
spec:
  replicas: 1
  selector:
    matchLabels:
      app: rust-web-demo-postgres
  template:
    metadata:
      labels:
        app: rust-web-demo-postgres
    spec:
      containers:
      - name: rust-web-demo-postgres
        image: postgres
        ports:
        - containerPort: 5432
        env:
        - name: POSTGRES_DB
          value: rust-web-demo
        - name: POSTGRES_USER
          value: diesel
        - name: POSTGRES_PASSWORD
          value: changeme
EOF
```

Note that you'll want to change the password value `changeme` to something else.

After you've saved the changes to the file, Skaffold will start updating your deployment, add your database and expose the database interally throughout the k8s cluster.

# Step 3 - Diesel

In this step we'll add [Diesel][16], an ORM for Rust which we'll use to interface with our [PostgreSQL][9] service.

## Database Proxy

For convenience, we'll set up a [port forward][17] to our PostgreSQL instance running in Kubernetes so we can deploy schema updates to it remotely. If you take this demo further, you may want to use something like [Kubernetes init containers][27] to run migrations.

Dedicate a terminal to running the forwarder. Run the following:

```
kubectl port-forward $(kubectl get pods|awk '/^rust-web-demo-postgres.*Running/{print$1}'|head) 5432:5432
```

(Note: you may occasionally need to re-run the above if for some reason the pod you're connected to goes away)

If you have `psql` installed locally, you'll now be able to access the database with the following line:

```
psql -U diesel -h localhost -p 5432 -d rust-web-demo
```

## Diesel Set Up & Configuration

Diesel comes with a [CLI][18] to help us manage our project, and makes it easy to generate, run, and revert database migrations.

### Diesel CLI

Install the [Diesel CLI][18] using `cargo` (you may need to install PostgreSQL development libraries on your system):

```
cargo install diesel_cli --no-default-features --features postgres
```

We'll also build a local `.env` file to instruct the Diesel CLI on how to access our database:

```
echo DATABASE_URL=postgres://diesel:changeme@localhost:5432/rust-web-demo > .env
```

Replacing `changeme` with whatever password you selected for your database.

### Diesel Setup

From here we can have Diesel set up our local environment and deploy a template for migrations:

```
diesel setup
```

The following files will be have been created:

* `migrations/00000000000000_diesel_initial_setup/up.sql`
* `migrations/00000000000000_diesel_initial_setup/down.sql`

These are our first in a series of migrations which we will use to grow and manage the database over time.

The `up.sql` will run to update our schema with changes, and when the `down.sql` it should cleanly remove those changes.

Overwrite `migrations/00000000000000_diesel_initial_setup/up.sql` and add the following contents:

```sql
cat <<'EOF' > migrations/00000000000000_diesel_initial_setup/up.sql
CREATE TABLE employees (
    id         SERIAL PRIMARY KEY,
    fname      VARCHAR NOT NULL,
    lname      VARCHAR NOT NULL,
    age        INTEGER NOT NULL,
    title      VARCHAR NOT NULL
);
EOF
```

Overwrite `migrations/00000000000000_diesel_initial_setup/down.sql` as well with these contents:

```sql
cat <<'EOF' > migrations/00000000000000_diesel_initial_setup/down.sql
DROP TABLE IF EXISTS employees;
DROP SEQUENCE IF EXISTS employees_id_seq;
EOF
```

### Migrations

Now that we have `up.sql` and `down.sql` set up, we can run our first migrations across the kubectl forwarder we set up to the database in Kubernetes:

```
diesel migration run
diesel migration redo
```

## Updating our Rust App with Diesel

At this point diesel_cli is installed, configured, and we've set up and run our first migration.

Our next step is to add code to use the database with Diesel and display database information with Rocket.

### PgConnection

Create a new file `src/postgres.rs` for providing a `PgConnection`, which we'll use to communicate with the database in our code:

```rust
cat <<'EOF' > src/postgres.rs
use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

pub fn connect() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}
EOF
```

### Models & Schema

We'll add our `Employee` model from the `employees` table created previously in a new file `src/models.rs`:

```rust
cat <<'EOF' > src/models.rs
#[derive(Queryable)]
pub struct Employee {
    pub id:    i32,
    pub fname: String,
    pub lname: String,
    pub age:   i32,
    pub title: String,
}
EOF
```

And we'll let Diesel generate our `src/schema.rs` file containing a macro automatic code generation and comprehension of our database schema:

```
diesel print-schema > src/schema.rs
```

This file will have the following contents:

```rust
table! {
    employees (id) {
        id -> Int4,
        fname -> Varchar,
        lname -> Varchar,
        age -> Int4,
        title -> Varchar,
    }
}
```

### Bringing it together

With the models, schema and connection code in place we can demonstrate our work.

#### Default Employee

We'll add a single employee manually so that we have some data present to work with by default:

```
psql -U diesel -h localhost -p 5432 -d rust-web-demo -c "INSERT INTO employees (id, fname, lname, age, title) VALUES (1, 'some', 'person', 25, 'Software Engineer');"
psql -U diesel -h localhost -p 5432 -d rust-web-demo -c "SELECT setval('employees_id_seq', 1, true);"
```

You should now be able to run `psql -U diesel -h localhost -p 5432 -d rust-web-demo -c 'SELECT * FROM employees'` and see the employee:

```
 id | fname | lname  | age |       title
----+-------+--------+-----+-------------------
  1 | some  | person |  25 | Software Engineer
(1 row)
```

#### Adding DATABASE_URL to rust-web-demo ENV

We'll add the `DATABASE_URL` to the environment variables for our app container in `skaffold-deployment.yaml` so that the app can use the db.

We'll need to add a [k8s secret][19] to store the sensitive value and use it in the container.

Add the following to the `skaffold-deployment.yaml`:

```yaml
cat <<'EOF' >> skaffold-deployment.yaml
---
apiVersion: v1
kind: Secret
metadata:
  name: rust-web-demo-database-url
type: Opaque
data:
  url: cG9zdGdyZXM6Ly9kaWVzZWw6Y2hhbmdlbWVAcnVzdC13ZWItZGVtby1wb3N0Z3Jlczo1NDMyL3J1c3Qtd2ViLWRlbW8=
EOF
```

The value for `key` is base64 encoded `DATABASE_URL` and the `value` is base64 of `postgres://diesel:changeme@rust-web-demo-postgres:5432/rust-web-demo`.

Now to [use the secret as an environment variable][20] you need to update the `rust-web-demo` container to look like this:

```yaml
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rust-web-demo
spec:
  replicas: 1
  selector:
    matchLabels:
      app: rust-web-demo
  template:
    metadata:
      labels:
        app: rust-web-demo
    spec:
      containers:
      - name: rust-web-demo
        image: docker.io/gkleiman/rust-web-demo
        ports:
        - containerPort: 8000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: rust-web-demo-database-url
              key: url
```

Note that when you save the file, this is going to trigger a rebuild of the PostgreSQL container via Skaffold so give it a few seconds.

#### Basic Display

We'll update our `src/main.rs` to provide some output that's actually pulled from the database now. Update `src.main.rs` with the following contents:

```rust
cat <<'EOF' > src/main.rs
#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate rocket;

mod postgres;
mod schema;
mod models;

use rocket::config::{Config, Environment};
use diesel::prelude::*;
use self::models::*;

#[get("/")]
fn hello() -> String {
    use self::schema::employees::dsl::*;

    let db = postgres::connect();
    let results = employees.filter(fname.eq("some"))
        .load::<Employee>(&db)
        .expect("Error loading Employees");

    format!("Default Employee: {} {}\n", results[0].fname, results[0].lname)
}

fn main() {
    let config = Config::build(Environment::Staging)
        .address("0.0.0.0")
        .port(8000)
        .finalize()
        .unwrap();

    rocket::custom(config, true)
        .mount("/", routes![hello])
        .launch();
}
EOF
```

After you write these changes, Skaffold will ship it off to the cluster and you'll soon be able to check it out with:

```
curl `minikube service rust-web-demo --url`
```

If everything is working you should receive:

```
Default Employee: some person
```

# REST API with Rocket

In this step we will expand our use of Rocket and Diesel to make a minimal demonstration [REST][21] API.

We will implement GET, PUT, POST, and DELETE methods which will utilize our PostgreSQL database.

You can freeze skaffold temporarily with `CTRL+Z` and resume it with `fg` after all the files here are updated.

## Updating Models

We need to derive several traits for our `Employee` model, including `Serialize` and `Deserialize` for working with the model in JSON, but also `Queryable` and `Insertable` to easily work with the model against the database.

Update the existing `src/models.rs` to contain the following contents:

```rust
cat <<'EOF' > src/models.rs
use schema::employees;

#[derive(Clone, Debug, Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "employees"]
pub struct Employee {
    pub id:    i32,
    pub fname: String,
    pub lname: String,
    pub age:   i32,
    pub title: String,
}

#[derive(Serialize, Deserialize)]
pub struct EmployeeList {
    pub results: Vec<Employee>,
}
EOF
```

Note that we also created `EmployeeList`, which will be used for producing multiple `Employee` results in API calls.

## Adding Errors

We're going to add a simple JSON serializable struct for handling ApiErrors, create the new file `src/errors.rs` with the following contents:

```rust
cat <<'EOF' > src/errors.rs
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}
EOF
```

## Adding Forms

We'll need a serializable [Rocket Form][22] to make it simple to accept `Employee` related parameters in HTTP requests for GET, PUT, POST, and DELETE.

Add the new file `src/forms.rs` with the following contents:

```rust
cat <<'EOF' > src/forms.rs
use schema::employees;

#[derive(Clone, Debug, Serialize, Deserialize, FromForm, Insertable, AsChangeset)]
#[table_name = "employees"]
pub struct EmployeeForm {
    pub id:    Option<i32>,
    pub fname: Option<String>,
    pub lname: Option<String>,
    pub age:   Option<i32>,
    pub title: Option<String>,
}
EOF
```

We've also implemented Insertable for this Form, as that will make it simple to use in database INSERTs.

## Adding the API HTTP Methods: GET, PUT, POST & DELETE

Now we can add our actual HTTP methods for the API so that we can GET, PUT, POST, and DELETE our employee data.

Add the new file `src/api.rs` with the following contents:

```rust
cat <<'EOF' > src/api.rs
use diesel::{delete, insert_into, update};
use diesel::prelude::*;

use rocket;
use rocket::{Catcher, Route, Request};
use rocket::response::status::{BadRequest, Created, NoContent};
use rocket_contrib::Json;

use errors::ApiError;
use forms::EmployeeForm;
use models::{Employee, EmployeeList};
use postgres::connect as dbc;

// -----------------------------------------------------------------------------
// HTTP Errors
// -----------------------------------------------------------------------------

#[catch(404)]
fn not_found(_: &Request) -> Json<ApiError> {
    Json(ApiError{
        message: "not found".to_string(),
    })
}

// -----------------------------------------------------------------------------
// HTTP GET, PUT, POST & DELETE
// -----------------------------------------------------------------------------

#[get("/employees", format = "application/json")]
fn employee_list() -> Json<EmployeeList> {
    use schema::employees::dsl::*;

    let db = dbc();
    let results = employees.load::<Employee>(&db)
        .expect("Error loading Employees");

    Json(EmployeeList {
        results: results.to_vec(),
    })
}

#[get("/employees/<employee_id>", format = "application/json")]
fn employee_get(employee_id: i32) -> Option<Json<Employee>> {
    use schema::employees::dsl::*;

    let db = dbc();
    match employees.find(employee_id).first::<Employee>(&db) {
        Ok(employee) => Some(Json(employee)),
        Err(_) => None,
    }
}

#[put("/employees", format = "application/json", data = "<json_employee>")]
fn employee_put(json_employee: Json<EmployeeForm>) -> Result<Created<()>, BadRequest<String>> {
    use schema::employees::dsl::*;

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

#[post("/employees/<employee_id>", format = "application/json", data = "<json_employee>")]
fn employee_update(employee_id: i32, json_employee: Json<EmployeeForm>) -> Result<NoContent, BadRequest<String>> {
    use schema::employees::dsl::*;

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

#[delete("/employees/<employee_id>", format = "application/json")]
fn employee_delete(employee_id: i32) -> Option<NoContent> {
    use schema::employees::dsl::*;

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
    routes![employee_list, employee_get, employee_put, employee_update, employee_delete]
}

pub fn gen_errors() -> Vec<Catcher> {
    catchers![not_found]
}
EOF
```

Note that there are some inefficiencies with our methods, such as creating a database connection for each method call. This was left simple for demonstration purposes, but we will revisit this and talk about some potential improvements (such as using Rocket's [Managed State][23] with [Diesel R2D2 connection pooling][24]) at the end.

# Bringing it all together

Now that we've added the API and several necessary pieces, we'll glue it all together in the main by updating `src/main.rs` to have the following contents:

```rust
cat <<'EOF' > src/main.rs
#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen)]

#[macro_use] extern crate diesel;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate rocket_contrib;
extern crate rocket;
extern crate dotenv;

mod api;
mod errors;
mod forms;
mod models;
mod postgres;
mod schema;

use rocket::config::{Config, Environment};

use api::{gen_routes, gen_errors};

fn main() {
    let config = Config::build(Environment::Staging)
        .address("0.0.0.0")
        .port(8000)
        .finalize()
        .unwrap();

    rocket::custom(config, true)
        .mount("/", gen_routes())
        .catch(gen_errors())
        .launch();
}
EOF
```

Once you've saved the changes to this file and all the other files added and modified above you can run `fg` to let skaffold deploy the new changes (if you froze it previously).

## Using the API

### GET

Once Skaffold has finished its work and your app is fully deployed, you should be able to GET the initial `Employee` from our migrations:

```
curl -s -w '\n%{http_code}\n' "$(minikube service rust-web-demo --url)/employees/1"
```

You should receive a 200 HTTP OK and the JSON of the `Employee`.

You should also be able to see the initial `Employee` in an `EmployeeList`:

```
curl -s -w '\n%{http_code}\n' "$(minikube service rust-web-demo --url)/employees"
```

You should receive a 200 HTTP OK and the JSON of the `EmployeeList`.

### PUT

You can add a new `Employee`:

```
curl -s -w '\n%{http_code}\n' -X PUT \
    -H 'Content-Type: application/json' \
    -d '{"fname":"new", "lname":"person", "age": 27, "title":"Devops Engineer"}' \
    "$(minikube service rust-web-demo --url)/employees"
```

You should receive a 201 Created.

### POST

You can update some information on that `Employee` with a POST:

```
curl -s -w '\n%{http_code}\n' -X POST \
    -H 'Content-Type: application/json' \
    -d '{"age": 29}' \
    "$(minikube service rust-web-demo --url)/employees/<employee_id>"
```

In the above `<employee_id>` will be whatever you received for the `id` in the PUT operation above (probably it will be 2 at this point unless you've done further experimentation).

Now you can get a new `EmployeeList` and see the previous entries plus your newly created (and updated) entry:

```
curl -s -w '\n%{http_code}\n' "$(minikube service rust-web-demo --url)/employees"
```

### DELETE

When you're done you can delete the created employee(s):

```
curl -s -w '\n%{http_code}\n' -X DELETE \
    -H 'Content-Type: application/json' \
    "$(minikube service rust-web-demo --url)/employees/<employee_id>"
```

# Cleanup & Conclusion

If you would like to cleanup the resources deployed in this demo you can use `CTRL+C` to stop skaffold, which will cause it to clean up it's resources.

In this demo we built and deployed an app on minikube, expanded on our app using Diesel and Rocket and watched Skaffold build and ship the results in the background while we were making changes. If you like building web applications with Diesel and Rocket, we recommend following up by reading the [Diesel Documentation][25] and the [Rocket Documentation][41] to continue learning.

Throughout our demonstration we did something things simply to avoid overcomplicating the code examples, if you decide you'd like to continue building off of the examples here for your own application you may want to look into using [Diesel Connection Pooling][24] to avoid separate connections for each request, and storing the connection pool via [Rocket Managed State][23]. You'll want to investigate some HA set ups for PostgreSQL, potentially something like [PostgreSQL XL][28]. You'll also want to develop some pagination layer on top of the GET methods in the examples, and implement further search functionality.

It's encouraged to read more on [Skaffold][0] to get to better know more of the options and features avaiable.

[0]:https://github.com/GoogleCloudPlatform/skaffold
[1]:https://www.rust-lang.org
[2]:https://kubernetes.io/docs/tasks/tools/install-minikube/
[3]:https://docs.docker.com/install/
[4]:https://www.rust-lang.org/install.html
[5]:https://kubernetes.io/docs/tasks/tools/install-kubectl/
[6]:https://kubernetes.io/docs/admin/service-accounts-admin/
[7]:https://rocket.rs
[8]:https://kubernetes.io/docs/concepts/workloads/controllers/deployment/
[9]:https://www.postgresql.org/
[10]:https://github.com/rust-lang/cargo/issues/2644
[11]:https://docs.docker.com/engine/reference/commandline/push/
[12]:https://docs.docker.com/engine/reference/commandline/images/
[13]:https://hub.docker.com/
[14]:https://docs.docker.com/engine/reference/commandline/login/
[15]:https://github.com/GoogleCloudPlatform/skaffold#installation
[16]:https://diesel.rs
[17]:https://kubernetes.io/docs/tasks/access-application-cluster/port-forward-access-application-cluster/
[18]:https://github.com/diesel-rs/diesel/tree/master/diesel_cli
[19]:https://kubernetes.io/docs/concepts/configuration/secret/
[20]:https://kubernetes.io/docs/concepts/configuration/secret/#using-secrets-as-environment-variables
[21]:https://en.wikipedia.org/wiki/Representational_state_transfer
[22]:https://rocket.rs/guide/requests/#forms
[23]:https://rocket.rs/guide/state/#managed-state
[24]:https://github.com/diesel-rs/r2d2-diesel
[25]:https://docs.diesel.rs/diesel/index.html
[26]:https://rustup.rs
[27]:https://kubernetes.io/docs/concepts/workloads/pods/init-containers/
[28]:https://www.postgres-xl.org/
[29]:https://docs.docker.com/develop/develop-images/multistage-build/
[30]:https://www.alpinelinux.org/
