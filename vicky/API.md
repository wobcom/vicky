# API 

Note: This is going to be replaced by an automatic generated Swagger API documentation.

## Prelude

This API is an JSON API, therefore you need to set `Content-Type` header accordingly. Also it will return only valid JSON data types.

## Tasks

### List All Tasks

`GET /api/v1/tasks` returns all tasks.  
#### Response 
```json
[
    {
        "id": "29f8f5f9-8513-4c8c-8dd9-0d6652e02bfd",
        "display_name": "Deployment 1",
        "status": {
            "state": "RUNNING"
        },
        "locks": [],
        "flake_ref": {
            "flake": "gitlab:wobcom/example",
            "args": []
        }
    }
]
```

### Create A Task

`POST /api/v1/tasks` creates a new task.

#### Request

```json
{
    "display_name": "Deployment 2",
    "locks": [],
    "flake_ref": {
        "flake": "gitlab:wobcom/example",
        "args": []
    },
    "features": []
}
```

#### Response

```json
{
    "id": "e9a7d00d-68a5-4fce-83b3-1eec31aac1fe",
    "status": {
        "state": "NEW"
    }
}
```

#### With Locks

```json
{
  "display_name": "Deployment 3",
  "locks": [
    {
      "name": "anything",
      "type": "WRITE"
    }
  ],
  "flake_ref": {
    "flake": "gitlab:wobcom/example",
    "args": []
  },
  "features": []
}
```
#### and required machine features
```json
{
  "display_name": "Deployment 3",
  "locks": [
    {
      "name": "also_anything",
      "type": "READ"
    },
    {
      "name": "also_anything",
      "type": "READ"
    },
    {
      "name": "another_thing",
      "type": "WRITE"
    }
  ],
  "flake_ref": {
    "flake": "gitlab:wobcom/example",
    "args": []
  },
  "features": [ "testfeature1", "anotherfeature" ]
}
```
### Claim A Task

`POST /api/v1/tasks/claim` claims the next new task available that is supported by the featureset of the fairy.

```json
{ "features": [ "feat1", "feat2" ] }
```

#### Response

If there is no new task available, the API will return null.

```json
null
```

If there is a new task available, the API will return the following:

```json
{
    "id": "cdcb2137-b419-4ec4-9dc5-dd65e24fb059",
    "display_name": "Deployment 3",
    "status": {
        "state": "RUNNING"
    },
    "locks": [],
    "flake_ref": {
        "flake": "gitlab:wobcom/example",
        "args": []
    }
}
```

### Log Output To A Task

`POST /api/v1/tasks/<UUID>/logs` saves `lines` from the json input into an S3 compatible bucket.

```json
{
    "lines": [
        "hallo",
        "welt"
    ]
}
```

#### Response

It will return `null`, if successful.

### Finish A Task

`POST /api/v1/tasks/<UUID>/finish` finishes a task with a certain result.

#### Request 

```json
{
    "result": {
        "result": "SUCCESS"
    }
}
```

#### Response
```json
{
    "id": "cdcb2137-b419-4ec4-9dc5-dd65e24fb059",
    "display_name": "Deployment 4",
    "status": {
        "state": "FINISHED",
        "result": "SUCCESS"
    },
    "locks": [],
    "flake_ref": {
        "flake": "gitlab:wobcom/example",
        "args": []
    }
}
```
