use etcd_client::{Client};
use rocket::{get, post, State, serde::json::Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vickylib::{documents::{Task, TaskStatus, TaskResult, FlakeRef, Lock, DocumentClient}, vicky::{scheduler::Scheduler, errors::{HTTPError, VickyError}}};

use crate::auth::{User, Machine};


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTaskNew {
    display_name: String,
    flake_ref: FlakeRef,
    locks: Vec<Lock>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTask {
    id: Uuid,
    status: TaskStatus,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTaskFinish {
    result: TaskResult,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct LogLines {
    lines: Vec<String>
}

#[get("/")]
pub async fn tasks_get_user(etcd: &State<Client>, _user: User) -> Result<Json<Vec<Task>>, VickyError> {
    let tasks: Vec<Task> = etcd.get_all_tasks().await?;
    Ok(Json(tasks))
}

#[get("/", rank=2)]
pub async fn tasks_get_machine(etcd: &State<Client>, _machine: Machine) -> Result<Json<Vec<Task>>, VickyError> {
    let tasks: Vec<Task> = etcd.get_all_tasks().await?;
    Ok(Json(tasks))
}



#[get("/<id>/logs")]
pub async fn tasks_get_logs(id: String, _user: User) -> Result<Json<LogLines>, VickyError> {
    let _task_uuid = Uuid::parse_str(&id)?;

    let test_logs = r#"[1;37m{
    [0m[34;1m"name"[0m[1;37m: [0m[0;32m"v2raya"[0m[1;37m,
    [0m[34;1m"version"[0m[1;37m: [0m[0;32m"0.1.0"[0m[1;37m,
    [0m[34;1m"private"[0m[1;37m: [0m[0;37mtrue[0m[1;37m,
    [0m[34;1m"license"[0m[1;37m: [0m[0;32m"GPL-3.0"[0m[1;37m,
    [0m[34;1m"scripts"[0m[1;37m: [0m[1;37m{
        [0m[34;1m"serve"[0m[1;37m: [0m[0;32m"vue-cli-service serve"[0m[1;37m,
        [0m[34;1m"build"[0m[1;37m: [0m[0;32m"vue-cli-service build"[0m[1;37m,
        [0m[34;1m"lint"[0m[1;37m: [0m[0;32m"vue-cli-service lint"[0m[1;37m
    [1;37m}[0m[1;37m,
    [0m[34;1m"dependencies"[0m[1;37m: [0m[1;37m{
        [0m[34;1m"@mdi/font"[0m[1;37m: [0m[0;32m"^5.8.55"[0m[1;37m,
        [0m[34;1m"@nuintun/qrcode"[0m[1;37m: [0m[0;32m"^3.3.0"[0m[1;37m,
        [0m[34;1m"@vue/babel-preset-app"[0m[1;37m: [0m[0;32m"^4.2.2"[0m[1;37m,
        [0m[34;1m"axios"[0m[1;37m: [0m[0;32m"^0.21.1"[0m[1;37m,
        [0m[34;1m"buefy"[0m[1;37m: [0m[0;32m"^0.9.22"[0m[1;37m,
        [0m[34;1m"clipboard"[0m[1;37m: [0m[0;32m"^2.0.4"[0m[1;37m,
        [0m[34;1m"dayjs"[0m[1;37m: [0m[0;32m"^1.10.6"[0m[1;37m,
        [0m[34;1m"js-base64"[0m[1;37m: [0m[0;32m"^2.5.1"[0m[1;37m,
        [0m[34;1m"nanoid"[0m[1;37m: [0m[0;32m"^3.1.23"[0m[1;37m,
        [0m[34;1m"normalize.css"[0m[1;37m: [0m[0;32m"^8.0.1"[0m[1;37m,
        [0m[34;1m"pace-js"[0m[1;37m: [0m[0;32m"^1.2.4"[0m[1;37m,
        [0m[34;1m"qrcode"[0m[1;37m: [0m[0;32m"^1.4.2"[0m[1;37m,
        [0m[34;1m"register-service-worker"[0m[1;37m: [0m[0;32m"^1.6.2"[0m[1;37m,
        [0m[34;1m"vue"[0m[1;37m: [0m[0;32m"^2.7.14"[0m[1;37m,
        [0m[34;1m"vue-i18n"[0m[1;37m: [0m[0;32m"^8.15.3"[0m[1;37m,
        [0m[34;1m"vue-router"[0m[1;37m: [0m[0;32m"^3.0.6"[0m[1;37m,
        [0m[34;1m"vue-virtual-scroller"[0m[1;37m: [0m[0;32m"^1.0.10"[0m[1;37m,
        [0m[34;1m"vuex"[0m[1;37m: [0m[0;32m"^3.0.1"[0m[1;37m,
        [0m[34;1m"webpack-iconfont-plugin-nodejs"[0m[1;37m: [0m[0;32m"^1.0.16"[0m[1;37m
    [1;37m}[0m[1;37m,
    [0m[34;1m"devDependencies"[0m[1;37m: [0m[1;37m{
        [0m[34;1m"@babel/core"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@babel/eslint-parser"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-babel"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-eslint"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-router"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-vuex"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-service"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/eslint-config-prettier"[0m[1;37m: [0m[0;32m"^5.0.0"[0m[1;37m,
        [0m[34;1m"css-loader"[0m[1;37m: [0m[0;32m"^5.2.0"[0m[1;37m,
        [0m[34;1m"eslint"[0m[1;37m: [0m[0;32m"^7.32.0"[0m[1;37m,
        [0m[34;1m"eslint-config-prettier"[0m[1;37m: [0m[0;32m"^8.3.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-prettier"[0m[1;37m: [0m[0;32m"^4.0.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-vue"[0m[1;37m: [0m[0;32m"^8.0.3"[0m[1;37m,
        [0m[34;1m"highlight.js"[0m[1;37m: [0m[0;32m"^11.4.0"[0m[1;37m,
        [0m[34;1m"prettier"[0m[1;37m: [0m[0;32m"^2.4.1"[0m[1;37m,
        [0m[34;1m"sass"[0m[1;37m: [0m[0;32m"^1.19.0"[0m[1;37m,
        [0m[34;1m"sass-loader"[0m[1;37m: [0m[0;32m"^8.0.0"[0m[1;37m,
        [0m[34;1m"terser-webpack-plugin"[0m[1;37m: [0m[0;32m"^5.3.6"[0m[1;37m,
        [0m[34;1m"urijs"[0m[1;37m: [0m[0;32m"^1.19.11"[0m[1;37m
        [0m[34;1m"@babel/core"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@babel/eslint-parser"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-babel"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-eslint"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-router"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-vuex"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-service"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/eslint-config-prettier"[0m[1;37m: [0m[0;32m"^5.0.0"[0m[1;37m,
        [0m[34;1m"css-loader"[0m[1;37m: [0m[0;32m"^5.2.0"[0m[1;37m,
        [0m[34;1m"eslint"[0m[1;37m: [0m[0;32m"^7.32.0"[0m[1;37m,
        [0m[34;1m"eslint-config-prettier"[0m[1;37m: [0m[0;32m"^8.3.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-prettier"[0m[1;37m: [0m[0;32m"^4.0.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-vue"[0m[1;37m: [0m[0;32m"^8.0.3"[0m[1;37m,
        [0m[34;1m"highlight.js"[0m[1;37m: [0m[0;32m"^11.4.0"[0m[1;37m,
        [0m[34;1m"prettier"[0m[1;37m: [0m[0;32m"^2.4.1"[0m[1;37m,
        [0m[34;1m"sass"[0m[1;37m: [0m[0;32m"^1.19.0"[0m[1;37m,
        [0m[34;1m"sass-loader"[0m[1;37m: [0m[0;32m"^8.0.0"[0m[1;37m,
        [0m[34;1m"terser-webpack-plugin"[0m[1;37m: [0m[0;32m"^5.3.6"[0m[1;37m,
        [0m[34;1m"urijs"[0m[1;37m: [0m[0;32m"^1.19.11"[0m[1;37m
        [0m[34;1m"@babel/core"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@babel/eslint-parser"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-babel"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-eslint"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-router"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-vuex"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-service"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/eslint-config-prettier"[0m[1;37m: [0m[0;32m"^5.0.0"[0m[1;37m,
        [0m[34;1m"css-loader"[0m[1;37m: [0m[0;32m"^5.2.0"[0m[1;37m,
        [0m[34;1m"eslint"[0m[1;37m: [0m[0;32m"^7.32.0"[0m[1;37m,
        [0m[34;1m"eslint-config-prettier"[0m[1;37m: [0m[0;32m"^8.3.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-prettier"[0m[1;37m: [0m[0;32m"^4.0.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-vue"[0m[1;37m: [0m[0;32m"^8.0.3"[0m[1;37m,
        [0m[34;1m"highlight.js"[0m[1;37m: [0m[0;32m"^11.4.0"[0m[1;37m,
        [0m[34;1m"prettier"[0m[1;37m: [0m[0;32m"^2.4.1"[0m[1;37m,
        [0m[34;1m"sass"[0m[1;37m: [0m[0;32m"^1.19.0"[0m[1;37m,
        [0m[34;1m"sass-loader"[0m[1;37m: [0m[0;32m"^8.0.0"[0m[1;37m,
        [0m[34;1m"terser-webpack-plugin"[0m[1;37m: [0m[0;32m"^5.3.6"[0m[1;37m,
        [0m[34;1m"urijs"[0m[1;37m: [0m[0;32m"^1.19.11"[0m[1;37m
        [0m[34;1m"@babel/core"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@babel/eslint-parser"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-babel"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-eslint"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-router"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-vuex"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-service"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/eslint-config-prettier"[0m[1;37m: [0m[0;32m"^5.0.0"[0m[1;37m,
        [0m[34;1m"css-loader"[0m[1;37m: [0m[0;32m"^5.2.0"[0m[1;37m,
        [0m[34;1m"eslint"[0m[1;37m: [0m[0;32m"^7.32.0"[0m[1;37m,
        [0m[34;1m"eslint-config-prettier"[0m[1;37m: [0m[0;32m"^8.3.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-prettier"[0m[1;37m: [0m[0;32m"^4.0.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-vue"[0m[1;37m: [0m[0;32m"^8.0.3"[0m[1;37m,
        [0m[34;1m"highlight.js"[0m[1;37m: [0m[0;32m"^11.4.0"[0m[1;37m,
        [0m[34;1m"prettier"[0m[1;37m: [0m[0;32m"^2.4.1"[0m[1;37m,
        [0m[34;1m"sass"[0m[1;37m: [0m[0;32m"^1.19.0"[0m[1;37m,
        [0m[34;1m"sass-loader"[0m[1;37m: [0m[0;32m"^8.0.0"[0m[1;37m,
        [0m[34;1m"terser-webpack-plugin"[0m[1;37m: [0m[0;32m"^5.3.6"[0m[1;37m,
        [0m[34;1m"urijs"[0m[1;37m: [0m[0;32m"^1.19.11"[0m[1;37m
        [0m[34;1m"@babel/core"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@babel/eslint-parser"[0m[1;37m: [0m[0;32m"^7.12.16"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-babel"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-eslint"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-router"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-plugin-vuex"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/cli-service"[0m[1;37m: [0m[0;32m"~5.0.8"[0m[1;37m,
        [0m[34;1m"@vue/eslint-config-prettier"[0m[1;37m: [0m[0;32m"^5.0.0"[0m[1;37m,
        [0m[34;1m"css-loader"[0m[1;37m: [0m[0;32m"^5.2.0"[0m[1;37m,
        [0m[34;1m"eslint"[0m[1;37m: [0m[0;32m"^7.32.0"[0m[1;37m,
        [0m[34;1m"eslint-config-prettier"[0m[1;37m: [0m[0;32m"^8.3.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-prettier"[0m[1;37m: [0m[0;32m"^4.0.0"[0m[1;37m,
        [0m[34;1m"eslint-plugin-vue"[0m[1;37m: [0m[0;32m"^8.0.3"[0m[1;37m,
        [0m[34;1m"highlight.js"[0m[1;37m: [0m[0;32m"^11.4.0"[0m[1;37m,
        [0m[34;1m"prettier"[0m[1;37m: [0m[0;32m"^2.4.1"[0m[1;37m,
        [0m[34;1m"sass"[0m[1;37m: [0m[0;32m"^1.19.0"[0m[1;37m,
        [0m[34;1m"sass-loader"[0m[1;37m: [0m[0;32m"^8.0.0"[0m[1;37m,
        [0m[34;1m"terser-webpack-plugin"[0m[1;37m: [0m[0;32m"^5.3.6"[0m[1;37m,
        [0m[34;1m"urijs"[0m[1;37m: [0m[0;32m"^1.19.11"[0m[1;37m
    [1;37m}[0m[1;37m
[1;37m}[0m[1;37m
    "#;

    let lines = LogLines{
        lines: test_logs.split('\n').map(ToString::to_string).collect(), 
    };
    
    Ok(Json(lines))
}


#[post("/claim")]
pub async fn tasks_claim(etcd: &State<Client>, _machine: Machine) ->  Result<Json<Option<Task>>, VickyError> {
    let tasks = etcd.get_all_tasks().await?;
    let scheduler = Scheduler::new(tasks)?;
    let next_task = scheduler.get_next_task();

    match next_task {
        Some(next_task) => {
            let mut task = etcd.get_task(next_task.id).await?.ok_or(HTTPError::NotFound)?;
            task.status = TaskStatus::RUNNING;
            etcd.put_task(&task).await?;
            Ok(Json(Some(task)))
        },
        None => Ok(Json(None)),
    }

   
}




#[post("/<id>/finish", format = "json", data = "<finish>")]
pub async fn tasks_finish(id: String, finish: Json<RoTaskFinish>, etcd: &State<Client>, _machine: Machine) ->  Result<Json<Task>, VickyError> {
    let task_uuid = Uuid::parse_str(&id)?;
    let mut task = etcd.get_task(task_uuid).await?.ok_or(HTTPError::NotFound)?; 
    task.status = TaskStatus::FINISHED(finish.result.clone());
    etcd.put_task(&task).await?;
    Ok(Json(task))
}



#[post("/", data = "<task>")]
pub async fn tasks_add(task: Json<RoTaskNew>, etcd: &State<Client>, _machine: Machine) -> Result<Json<RoTask>, VickyError> {
    let task_uuid = Uuid::new_v4();

    let task_manifest = Task { 
        id: task_uuid,
        status: TaskStatus::NEW,
        locks: task.locks.clone(),
        display_name: task.display_name.clone(),
        flake_ref: FlakeRef { flake: task.flake_ref.flake.clone(), args: task.flake_ref.args.clone() },
    };

    etcd.put_task(&task_manifest).await?;

    let ro_task = RoTask {
        id: task_uuid,
        status: TaskStatus::NEW
    };

    Ok(Json(ro_task))

}
