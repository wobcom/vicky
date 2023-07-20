# Vicky

Vicky, which is the babysitter of Timmy, Cosmo and Wanda, is a CD tool for environments with many constraints and dependencies that usually cannot be represented.

## Background

We use an etcd cluster to sync state between multiple instances of Vicky. Vicky will do leader election, so at each time only one instance is active. We try to make Vicky as resilient to network and other failues as possible but it is not our main goal, yet.
All data in the etcd is stored under `vicky.wobcom.de/` in YAML format. 

There will be two binaries, `vicky` and `vickyctl`, similar to `kubelet` and `kubectl` in the Kubernetes world. `vicky` is the main continous delivery tool, which works only on data within etcd. `vickyctl` can be used to manipulate certain data within etcd, e.g. adding or deleting a node from `vicky`.

## Usage

### Add a node

You need to create a node manifest. An example is located within `./manifests`.
This node manifest can be added to vicky by using `vickyctl node apply manifests/node.example.yaml`.

### Deleting a node

This node manifest can be deleted from vicky by using `vickyctl node delete manifests/node.example.yaml`.


