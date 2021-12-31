# Deploying Rust in Kubernetes (minikube) with Skaffold [Actual on january 2022] 

[YouTube conference recording](https://www.youtube.com/watch?v=9S1-69Rp1vQ)


<br/>

## Run Local

<br/>

```
$ sudo apt-get install libpq-dev jq
```

<br/>

```
$ sudo vi /etc/hosts
```

<br/>

```
127.0.0.1 postgres
```

<br/>

```
// run database
$ docker-compose up postgres
```

<br/>


```
$ cargo install diesel_cli --no-default-features --features postgres
$ diesel migration run
```

<br/>

```
$ cargo run
```


<br/>

## Run in docker

```
// compile app
$ docker-compose up --build

```

<br/>

## Using the API


### PUT

```
// ADD NEW
$ curl -s -w '\n%{http_code}\n' -X PUT \
    -H 'Content-Type: application/json' \
    -d '{"fname":"new", "lname":"person", "age": 27, "title":"Devops Engineer"}' \
    "localhost:8000/employees"
```

### GET

```
// GET ALL
$ curl -s -w '\n%{http_code}\n' "localhost:8000/employees" | jq
```

```
// GET BY ID
$ curl -s -w '\n%{http_code}\n' "localhost:8000/employees/<employee_id>"
```

### POST

```
// UPDATE
$ curl -s -w '\n%{http_code}\n' -X POST \
    -H 'Content-Type: application/json' \
    -d '{"age": 29}' \
    "http://localhost:8000/employees/<employee_id>"
```

### DELETE

```
// DELETE
$ curl -s -w '\n%{http_code}\n' -X DELETE \
    -H 'Content-Type: application/json' \
    "http://localhost:8000/employees/<employee_id>"
```

<br/>

## Run in kubernetes

<br/>

https://shell.cloud.google.com/

<br/>

```
$ gcloud auth login
$ gcloud cloud-shell ssh
```


<br/>

```
// Rust installation
$ cd ~/tmp/
$ curl https://sh.rustup.rs -sSf | sh

$ source $HOME/.cargo/env

$ rustup update

$ rustc --version
$ cargo --version
```

<br/>

```
$ sudo apt install -y iputils-ping jq
```

<br/>

```
$ cargo install diesel_cli --no-default-features --features postgres
```

<br/>

```
$ export \
    PROFILE=${USER}-minikube \
    MEMORY=8192 \
    CPUS=4 \
    DRIVER=docker \
    KUBERNETES_VERSION=v1.23.1
```

<br/>

[Run minikube in free google clouds](//gitops.ru/containers/kubernetes/minikube/setup/)

<br/>

```
$ cd ~/tmp
$ git clone https://github.com/webmakaka/k8s-rust-skaffold-demo

$ cd k8s-rust-skaffold-demo/skaffold

$ skaffold dev
```

<br/>

```
$ kubectl get pods
NAME                                      READY   STATUS    RESTARTS   AGE
rust-web-demo-6d5c4b5f84-4lcjx            1/1     Running   0          15s
rust-web-demo-6d5c4b5f84-lc82j            1/1     Running   0          15s
rust-web-demo-6d5c4b5f84-tm2xx            1/1     Running   0          15s
rust-web-demo-postgres-797b7c9f67-nngrg   1/1     Running   0          15s
```


<br/>

```
$ kubectl port-forward $(kubectl get pods|awk '/^rust-web-demo-postgres.*Running/{print$1}'|head) 5432:5432
```

<br/>

```
$ cd ~/tmp/k8s-rust-skaffold-demo/app/
```

<br/>

```
$ diesel migration run
```

<br/>

```
// ADD DATA
$ psql -U diesel -h localhost -p 5432 -d rust-web-demo -c "INSERT INTO employees (id, fname, lname, age, title) VALUES (1, 'some', 'person', 25, 'Software Engineer');"
```

<br/>

```
// CHECK DATA
$ psql -U diesel -h localhost -p 5432 -d rust-web-demo -c 'SELECT * FROM employees'
```

<br/>

```
 id | fname | lname  | age |       title       
----+-------+--------+-----+-------------------
  1 | some  | person |  25 | Software Engineer
(1 row)
```

<br/>

```
$ minikube --profile ${PROFILE} ip
192.168.49.2
```

<br/>

```
$ export INGRESS_HOST=192.168.49.2.nip.io
```

<br/>

## Using the API


### GET


```
// GET ALL
$ curl -s -w '\n%{http_code}\n' "${INGRESS_HOST}/employees" | jq
```

<br/>

**returns:**

```
{
  "results": [
    {
      "id": 1,
      "fname": "some",
      "lname": "person",
      "age": 25,
      "title": "Software Engineer"
    }
  ]
}
200

```

<br/>


```
// GET BY ID
$ curl -s -w '\n%{http_code}\n' "localhost:8000/employees/<employee_id>"
```

### PUT

```
// ADD NEW
$ curl -s -w '\n%{http_code}\n' -X PUT \
    -H 'Content-Type: application/json' \
    -d '{"fname":"new", "lname":"person", "age": 27, "title":"Devops Engineer"}' \
    "${INGRESS_HOST}/employees"
```

### POST

```
// UPDATE
$ curl -s -w '\n%{http_code}\n' -X POST \
    -H 'Content-Type: application/json' \
    -d '{"age": 29}' \
    "http://localhost:8000/employees/<employee_id>"
```

### DELETE

```
// DELETE
$ curl -s -w '\n%{http_code}\n' -X DELETE \
    -H 'Content-Type: application/json' \
    "http://localhost:8000/employees/<employee_id>"
```

<br/>

### P.S.

**secret.yaml**

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: rust-web-demo-database-url
type: Opaque
data:
  url: cG9zdGdyZXM6Ly9kaWVzZWw6cEA1NXcwcmQxQHJ1c3Qtd2ViLWRlbW8tcG9zdGdyZXM6NTQzMi9ydXN0LXdlYi1kZW1v
```

<br/>

The value for `key` is base64 encoded `DATABASE_URL` and the `value` is base64 of `postgres://diesel:p@55w0rd1@rust-web-demo-postgres:5432/rust-web-demo`.

